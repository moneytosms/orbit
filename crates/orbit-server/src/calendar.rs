//! Read-only calendars via secret iCal feed URLs (several, each with name/color/visibility): storage,
//! SSRF-safe fetch, poll, endpoints.
//!
//! The URL is a secret: it is never logged, never put in an error, and clients only see
//! `scheme://host/…`. Events live in `calendar_events` (keyed by `feed_id`), outside the synced row store.

use std::{
    future::Future,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use reqwest::{Url, redirect::Policy};
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{St, auth, ical, lock, now_ms};

const DAY_MS: i64 = 86_400_000;
/// How often the poll job refreshes the feeds.
pub const POLL_INTERVAL: Duration = Duration::from_secs(30 * 60);
/// Days before now and after now that occurrences are expanded and stored.
const PAST_DAYS: i64 = 31;
const FUTURE_DAYS: i64 = 400;
const MAX_BYTES: usize = 5 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(15);
const MAX_REDIRECTS: usize = 3;
const MAX_ROWS: i64 = 5_000;
/// Feeds a user may add.
const MAX_FEEDS: i64 = 10;
/// Design-token names (see app.css); the client paints `var(--<color>)`, so themes and dark mode just work.
const COLORS: [&str; 6] = ["college", "career", "learning", "home", "accent", "ok"];

/// Fetches a feed URL to bytes. Tests swap in a fake; the real one is `fetch_feed`.
pub type Fetcher = Arc<
    dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + Send>> + Send + Sync,
>;

pub fn real_fetcher() -> Fetcher {
    Arc::new(|url| Box::pin(fetch_feed(url)))
}

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS calendar_feeds (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, url TEXT NOT NULL, color TEXT NOT NULL, visible INTEGER NOT NULL DEFAULT 1, last_sync INTEGER, last_error TEXT);
         CREATE TABLE IF NOT EXISTS calendar_events (feed_id INTEGER NOT NULL DEFAULT 0, uid TEXT NOT NULL, summary TEXT NOT NULL, start_ms INTEGER NOT NULL, end_ms INTEGER NOT NULL, all_day INTEGER NOT NULL, start_date TEXT, end_date TEXT);",
    )?;
    // Events from the single-feed version lack the new columns.
    let has = |t: &str, c: &str| -> rusqlite::Result<bool> {
        conn.query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info(?1) WHERE name = ?2",
            [t, c],
            |r| r.get(0),
        )
    };
    if !has("calendar_events", "feed_id")? {
        conn.execute_batch(
            "ALTER TABLE calendar_events ADD COLUMN feed_id INTEGER NOT NULL DEFAULT 0;
             ALTER TABLE calendar_events ADD COLUMN start_date TEXT;
             ALTER TABLE calendar_events ADD COLUMN end_date TEXT;
             UPDATE calendar_events SET start_date = date(start_ms / 1000, 'unixepoch'), end_date = date(end_ms / 1000, 'unixepoch') WHERE all_day = 1;",
        )?;
    }
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS calendar_events_start ON calendar_events (start_ms);
         CREATE INDEX IF NOT EXISTS calendar_events_feed ON calendar_events (feed_id);",
    )?;
    // The single `calendar_feed` row becomes feed "Calendar" and adopts the stored events.
    let legacy: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type = 'table' AND name = 'calendar_feed'",
        [],
        |r| r.get(0),
    )?;
    if legacy {
        conn.execute_batch(
            "BEGIN;
             INSERT INTO calendar_feeds (name, url, color, visible, last_sync, last_error)
               SELECT 'Calendar', url, 'college', 1, last_sync, last_error FROM calendar_feed WHERE url IS NOT NULL;
             DELETE FROM calendar_events WHERE feed_id = 0 AND NOT EXISTS (SELECT 1 FROM calendar_feeds);
             UPDATE calendar_events SET feed_id = (SELECT MAX(id) FROM calendar_feeds) WHERE feed_id = 0;
             DROP TABLE calendar_feed;
             COMMIT;",
        )?;
    }
    Ok(())
}

// ---- SSRF ----

fn public_v4(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    !(ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_multicast()
        || o[0] == 0
        || o[0] >= 240
        || (o[0] == 100 && (64..128).contains(&o[1]))
        || (o[0] == 192 && o[1] == 0 && o[2] == 0)
        || (o[0] == 192 && o[1] == 0 && o[2] == 2)
        || (o[0] == 198 && (o[1] == 18 || o[1] == 19))
        || (o[0] == 198 && o[1] == 51 && o[2] == 100)
        || (o[0] == 203 && o[1] == 0 && o[2] == 113))
}

fn public_v6(ip: Ipv6Addr) -> bool {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return public_v4(v4);
    }
    let s = ip.segments();
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || s[0] & 0xfe00 == 0xfc00 // unique local
        || s[0] & 0xffc0 == 0xfe80 // link local
        || (s[0] == 0x64 && s[1] == 0xff9b) // NAT64 can reach v4 private space
        || (s[0] == 0x2001 && s[1] == 0xdb8)) // documentation
}

pub fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => public_v4(v),
        IpAddr::V6(v) => public_v6(v),
    }
}

/// Static checks on a feed URL: https, default port, no userinfo, host not a private IP literal.
/// DNS results are checked at fetch time.
pub fn check_url(raw: &str) -> Result<Url, &'static str> {
    let url = Url::parse(raw.trim()).map_err(|_| "not a valid URL")?;
    if url.scheme() != "https" {
        return Err("only https URLs are allowed");
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("URL must not contain credentials");
    }
    if url.port().is_some() {
        return Err("custom ports are not allowed");
    }
    match url.host() {
        None => Err("URL has no host"),
        Some(url::Host::Ipv4(ip)) if !public_v4(ip) => Err("address is not public"),
        Some(url::Host::Ipv6(ip)) if !public_v6(ip) => Err("address is not public"),
        Some(_) => Ok(url),
    }
}

/// `scheme://host/…`: enough to recognise the feed, nothing secret.
pub fn mask(raw: &str) -> String {
    Url::parse(raw).map_or_else(
        |_| "…".into(),
        |u| format!("{}://{}/…", u.scheme(), u.host_str().unwrap_or("")),
    )
}

async fn fetch_feed(raw: String) -> Result<Vec<u8>, String> {
    let mut url = check_url(&raw).map_err(String::from)?;
    for _ in 0..=MAX_REDIRECTS {
        let mut client = reqwest::Client::builder()
            .redirect(Policy::none())
            .timeout(TIMEOUT);
        if let Some(url::Host::Domain(host)) = url.host() {
            let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host, 443))
                .await
                .map_err(|_| "DNS lookup failed".to_string())?
                .collect();
            if addrs.is_empty() || !addrs.iter().all(|a| public_ip(a.ip())) {
                return Err("host resolves to a non-public address".into());
            }
            // Connect to exactly the addresses just checked (no re-resolve, so no DNS rebinding).
            client = client.resolve_to_addrs(host, &addrs);
        }
        let mut res = client
            .build()
            .map_err(|e| e.without_url().to_string())?
            .get(url.clone())
            .send()
            .await
            .map_err(|e| e.without_url().to_string())?;
        if res.status().is_redirection() {
            let next = res
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|l| url.join(l).ok())
                .ok_or("bad redirect")?;
            url = check_url(next.as_str()).map_err(|e| format!("redirect rejected: {e}"))?;
            continue;
        }
        if !res.status().is_success() {
            return Err(format!("feed returned HTTP {}", res.status().as_u16()));
        }
        let mut body = Vec::new();
        while let Some(chunk) = res.chunk().await.map_err(|e| e.without_url().to_string())? {
            body.extend_from_slice(&chunk);
            if body.len() > MAX_BYTES {
                return Err("feed is too large".into());
            }
        }
        return Ok(body);
    }
    Err("too many redirects".into())
}

// ---- refresh + poll ----

fn record_error(db: &Mutex<Connection>, id: i64, msg: &str) {
    let _ = lock(db).execute(
        "UPDATE calendar_feeds SET last_error = ?2 WHERE id = ?1",
        (id, msg),
    );
}

/// Fetches one feed and replaces its stored events. On failure its old events stay and `last_error`
/// is set; `last_sync` only moves on success. Other feeds are never touched. Unknown id: does nothing.
pub async fn refresh_feed(db: &Mutex<Connection>, fetch: &Fetcher, id: i64, now: i64) {
    let Ok(url) = lock(db).query_row("SELECT url FROM calendar_feeds WHERE id = ?1", [id], |r| {
        r.get::<_, String>(0)
    }) else {
        return;
    };
    let body = match fetch(url).await {
        Ok(b) => b,
        Err(e) => return record_error(db, id, &e),
    };
    let Ok(text) = String::from_utf8(body) else {
        return record_error(db, id, "feed is not UTF-8 text");
    };
    if !text.contains("BEGIN:VCALENDAR") {
        return record_error(db, id, "response is not an iCalendar feed");
    }
    let events = ical::parse_feed(&text, now - PAST_DAYS * DAY_MS, now + FUTURE_DAYS * DAY_MS);
    let mut conn = lock(db);
    let res = (|| {
        let tx = conn.transaction()?;
        // The feed may have been removed while fetching: store nothing then.
        let n = tx.execute(
            "UPDATE calendar_feeds SET last_sync = ?2, last_error = NULL WHERE id = ?1",
            (id, now),
        )?;
        if n == 0 {
            return Ok(());
        }
        tx.execute("DELETE FROM calendar_events WHERE feed_id = ?1", [id])?;
        for e in &events {
            tx.execute(
                "INSERT INTO calendar_events (feed_id, uid, summary, start_ms, end_ms, all_day, start_date, end_date) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                (id, &e.uid, &e.summary, e.start_ms, e.end_ms, e.all_day, &e.start_date, &e.end_date),
            )?;
        }
        tx.commit()
    })();
    if res.is_err() {
        drop(conn);
        record_error(db, id, "could not store events");
    }
}

/// Refreshes every feed, one after another; a failing feed only records its own error.
pub async fn refresh_all(db: &Mutex<Connection>, fetch: &Fetcher, now: i64) {
    let ids: Vec<i64> = {
        let conn = lock(db);
        let Ok(mut stmt) = conn.prepare("SELECT id FROM calendar_feeds ORDER BY id") else {
            return;
        };
        let Ok(rows) = stmt.query_map([], |r| r.get(0)) else {
            return;
        };
        rows.flatten().collect()
    };
    for id in ids {
        refresh_feed(db, fetch, id, now).await;
    }
}

/// Refreshes all feeds now and then every `POLL_INTERVAL`, for the life of the server.
pub async fn poll(st: St) {
    loop {
        refresh_all(&st.db, &st.fetcher, now_ms()).await;
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

// ---- endpoints ----

type ApiErr = (StatusCode, Json<Value>);

fn err(code: StatusCode, msg: &str) -> ApiErr {
    (code, Json(json!({ "error": msg })))
}

fn internal(e: rusqlite::Error) -> ApiErr {
    eprintln!("calendar: {e}");
    err(StatusCode::INTERNAL_SERVER_ERROR, "storage failed")
}

/// Feeds with health. The URL appears only masked.
fn feeds_json(conn: &Connection, only: Option<i64>) -> rusqlite::Result<Vec<Value>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, url, color, visible, last_sync, last_error,
                (SELECT COUNT(*) FROM calendar_events e WHERE e.feed_id = f.id)
         FROM calendar_feeds f WHERE ?1 IS NULL OR id = ?1 ORDER BY id",
    )?;
    stmt.query_map([only], |r| {
        Ok(json!({
            "id": r.get::<_, i64>(0)?,
            "name": r.get::<_, String>(1)?,
            "url_masked": mask(&r.get::<_, String>(2)?),
            "color": r.get::<_, String>(3)?,
            "visible": r.get::<_, bool>(4)?,
            "last_sync_ms": r.get::<_, Option<i64>>(5)?,
            "last_error": r.get::<_, Option<String>>(6)?,
            "event_count": r.get::<_, i64>(7)?,
        }))
    })
    .and_then(|it| it.collect())
}

fn one(conn: &Connection, id: i64) -> Result<Json<Value>, ApiErr> {
    feeds_json(conn, Some(id))
        .map_err(internal)?
        .pop()
        .map(Json)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such calendar"))
}

fn clean_name(name: &str) -> Result<&str, ApiErr> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > 60 {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "name must be 1 to 60 characters",
        ));
    }
    Ok(n)
}

fn clean_color(color: &str) -> Result<&str, ApiErr> {
    if COLORS.contains(&color) {
        Ok(color)
    } else {
        Err(err(StatusCode::BAD_REQUEST, "unknown color"))
    }
}

pub(crate) async fn list(
    State(st): State<St>,
    _: auth::Device,
) -> Result<Json<Vec<Value>>, ApiErr> {
    feeds_json(&lock(&st.db), None).map(Json).map_err(internal)
}

#[derive(Deserialize)]
pub(crate) struct NewFeed {
    name: String,
    url: String,
    color: Option<String>,
}

/// Adds a feed, refreshes it once, and returns it (a failed fetch shows up in its health, not as an error).
pub(crate) async fn create(
    State(st): State<St>,
    _: auth::Device,
    Json(body): Json<NewFeed>,
) -> Result<Json<Value>, ApiErr> {
    let name = clean_name(&body.name)?;
    let url = check_url(&body.url).map_err(|e| err(StatusCode::BAD_REQUEST, e))?;
    let id = {
        let conn = lock(&st.db);
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM calendar_feeds", [], |r| r.get(0))
            .map_err(internal)?;
        if n >= MAX_FEEDS {
            return Err(err(StatusCode::BAD_REQUEST, "at most 10 calendars"));
        }
        let color = match body.color.as_deref() {
            Some(c) => clean_color(c)?,
            None => COLORS[n as usize % COLORS.len()],
        };
        conn.execute(
            "INSERT INTO calendar_feeds (name, url, color) VALUES (?1, ?2, ?3)",
            (name, url.as_str(), color),
        )
        .map_err(internal)?;
        conn.last_insert_rowid()
    };
    refresh_feed(&st.db, &st.fetcher, id, now_ms()).await;
    one(&lock(&st.db), id)
}

#[derive(Deserialize)]
pub(crate) struct Patch {
    name: Option<String>,
    color: Option<String>,
    visible: Option<bool>,
}

pub(crate) async fn update(
    State(st): State<St>,
    _: auth::Device,
    Path(id): Path<i64>,
    Json(p): Json<Patch>,
) -> Result<Json<Value>, ApiErr> {
    let name = p.name.as_deref().map(clean_name).transpose()?;
    let color = p.color.as_deref().map(clean_color).transpose()?;
    let conn = lock(&st.db);
    conn.execute(
        "UPDATE calendar_feeds SET name = COALESCE(?2, name), color = COALESCE(?3, color), visible = COALESCE(?4, visible) WHERE id = ?1",
        (id, name, color, p.visible),
    )
    .map_err(internal)?;
    one(&conn, id)
}

pub(crate) async fn remove(
    State(st): State<St>,
    _: auth::Device,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiErr> {
    let mut conn = lock(&st.db);
    let tx = conn.transaction().map_err(internal)?;
    tx.execute("DELETE FROM calendar_events WHERE feed_id = ?1", [id])
        .map_err(internal)?;
    let n = tx
        .execute("DELETE FROM calendar_feeds WHERE id = ?1", [id])
        .map_err(internal)?;
    tx.commit().map_err(internal)?;
    if n == 0 {
        return Err(err(StatusCode::NOT_FOUND, "no such calendar"));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub(crate) struct Window {
    from: i64,
    to: i64,
}

/// Events of visible feeds overlapping `[from, to)` (epoch ms), oldest first. All-day events match on
/// their UTC-midnight sort keys, so callers pad the window by a day and pick the local day from
/// `start_date`/`end_date`.
pub(crate) async fn events(
    State(st): State<St>,
    _: auth::Device,
    Query(w): Query<Window>,
) -> Result<Json<Vec<Value>>, ApiErr> {
    let conn = lock(&st.db);
    let mut stmt = conn
        .prepare(
            "SELECT e.uid, e.summary, e.start_ms, e.end_ms, e.all_day, e.start_date, e.end_date, f.id, f.name, f.color
             FROM calendar_events e JOIN calendar_feeds f ON f.id = e.feed_id
             WHERE f.visible = 1 AND e.start_ms < ?2 AND MAX(e.end_ms, e.start_ms + 1) > ?1
             ORDER BY e.start_ms, e.uid LIMIT ?3",
        )
        .map_err(internal)?;
    let rows = stmt
        .query_map((w.from, w.to, MAX_ROWS), |r| {
            Ok(json!({
                "uid": r.get::<_, String>(0)?,
                "summary": r.get::<_, String>(1)?,
                "start_ms": r.get::<_, i64>(2)?,
                "end_ms": r.get::<_, i64>(3)?,
                "all_day": r.get::<_, bool>(4)?,
                "start_date": r.get::<_, Option<String>>(5)?,
                "end_date": r.get::<_, Option<String>>(6)?,
                "feed_id": r.get::<_, i64>(7)?,
                "feed_name": r.get::<_, String>(8)?,
                "color": r.get::<_, String>(9)?,
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(internal)?;
    Ok(Json(rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_url_accepts_plain_https() {
        assert!(
            check_url("https://calendar.google.com/calendar/ical/x/private-abc/basic.ics").is_ok()
        );
    }

    #[test]
    fn check_url_rejects_bad_schemes_ports_and_userinfo() {
        for u in [
            "http://calendar.google.com/a.ics",
            "ftp://calendar.google.com/a.ics",
            "file:///etc/passwd",
            "https://user:pw@calendar.google.com/a.ics",
            "https://calendar.google.com@127.0.0.1/a.ics",
            "https://calendar.google.com:8443/a.ics",
            "calendar.google.com/a.ics",
            "",
        ] {
            assert!(check_url(u).is_err(), "{u}");
        }
    }

    #[test]
    fn check_url_rejects_non_public_ip_literals_in_every_spelling() {
        for u in [
            "https://127.0.0.1/a.ics",
            "https://127.1/a.ics",
            "https://2130706433/a.ics",
            "https://0x7f.0.0.1/a.ics",
            "https://10.0.0.5/a.ics",
            "https://172.16.0.1/a.ics",
            "https://192.168.1.1/a.ics",
            "https://169.254.169.254/latest/meta-data",
            "https://100.64.0.1/a.ics",
            "https://0.0.0.0/a.ics",
            "https://[::1]/a.ics",
            "https://[::]/a.ics",
            "https://[fe80::1]/a.ics",
            "https://[fd00::1]/a.ics",
            "https://[::ffff:127.0.0.1]/a.ics",
            "https://[::ffff:10.0.0.1]/a.ics",
            "https://[64:ff9b::7f00:1]/a.ics",
        ] {
            assert!(check_url(u).is_err(), "{u}");
        }
        assert!(check_url("https://8.8.8.8/a.ics").is_ok());
        assert!(check_url("https://[2606:4700::1111]/a.ics").is_ok());
    }

    #[test]
    fn public_ip_covers_resolved_addresses() {
        for ip in [
            "127.0.0.1",
            "::1",
            "10.1.2.3",
            "192.168.0.9",
            "169.254.1.1",
            "fe80::1",
            "fc00::1",
        ] {
            assert!(!public_ip(ip.parse().unwrap()), "{ip}");
        }
        assert!(public_ip("142.250.80.46".parse().unwrap()));
    }

    #[test]
    fn mask_keeps_only_scheme_and_host() {
        let m =
            mask("https://calendar.google.com/calendar/ical/me%40x.com/private-SECRET/basic.ics");
        assert_eq!(m, "https://calendar.google.com/…");
        assert!(!m.contains("SECRET"));
    }

    fn fake(result: Result<&'static str, &'static str>) -> Fetcher {
        Arc::new(move |_| {
            Box::pin(async move { result.map(|s| s.as_bytes().to_vec()).map_err(String::from) })
        })
    }

    fn db() -> Mutex<Connection> {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        Mutex::new(conn)
    }

    fn add(db: &Mutex<Connection>, name: &str, url: &str) -> i64 {
        let c = lock(db);
        c.execute(
            "INSERT INTO calendar_feeds (name, url, color) VALUES (?1, ?2, 'college')",
            (name, url),
        )
        .unwrap();
        c.last_insert_rowid()
    }

    const FEED: &str = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:a\r\nDTSTART:20240101T100000Z\r\nRRULE:FREQ=DAILY;COUNT=3\r\nSUMMARY:S\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    const NOW: i64 = 1_704_067_200_000; // 2024-01-01T00:00:00Z

    fn state(db: &Mutex<Connection>, id: i64) -> (i64, Option<i64>, Option<String>) {
        let c = lock(db);
        let n = c
            .query_row(
                "SELECT COUNT(*) FROM calendar_events WHERE feed_id = ?1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        let (s, e) = c
            .query_row(
                "SELECT last_sync, last_error FROM calendar_feeds WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        (n, s, e)
    }

    #[tokio::test]
    async fn refresh_stores_events_and_records_last_sync() {
        let db = db();
        let id = add(&db, "A", "https://example.com/secret.ics");
        refresh_feed(&db, &fake(Ok(FEED)), id, NOW).await;
        assert_eq!(state(&db, id), (3, Some(NOW), None));
    }

    #[tokio::test]
    async fn failed_refresh_keeps_old_events_and_records_error_without_moving_last_sync() {
        let db = db();
        let id = add(&db, "A", "https://example.com/secret.ics");
        refresh_feed(&db, &fake(Ok(FEED)), id, NOW).await;
        refresh_feed(&db, &fake(Err("feed returned HTTP 500")), id, NOW + 1).await;
        assert_eq!(
            state(&db, id),
            (3, Some(NOW), Some("feed returned HTTP 500".into()))
        );
        refresh_feed(&db, &fake(Ok("<html>nope</html>")), id, NOW + 2).await;
        assert_eq!(
            state(&db, id).2.as_deref(),
            Some("response is not an iCalendar feed")
        );
        refresh_feed(&db, &fake(Ok(FEED)), id, NOW + 3).await;
        assert_eq!(state(&db, id), (3, Some(NOW + 3), None));
    }

    #[tokio::test]
    async fn one_failing_feed_leaves_the_others_and_its_own_old_events_intact() {
        let db = db();
        let good = add(&db, "Good", "https://good.example.com/a.ics");
        let bad = add(&db, "Bad", "https://bad.example.com/b.ics");
        refresh_all(&db, &fake(Ok(FEED)), NOW).await;
        assert_eq!(state(&db, good).0, 3);
        assert_eq!(state(&db, bad).0, 3);
        // Second poll: bad host fails, good host serves a one-event feed.
        let one = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:z\r\nDTSTART:20240101T100000Z\r\nSUMMARY:Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let fetch: Fetcher = Arc::new(move |url| {
            Box::pin(async move {
                if url.contains("bad.") {
                    Err("feed returned HTTP 500".to_string())
                } else {
                    Ok(one.as_bytes().to_vec())
                }
            })
        });
        refresh_all(&db, &fetch, NOW + 1).await;
        assert_eq!(state(&db, good), (1, Some(NOW + 1), None));
        assert_eq!(
            state(&db, bad),
            (3, Some(NOW), Some("feed returned HTTP 500".into()))
        );
    }

    #[tokio::test]
    async fn refresh_of_an_unknown_feed_never_fetches() {
        let db = db();
        let fetch: Fetcher = Arc::new(|_| panic!("fetched without a feed"));
        refresh_feed(&db, &fetch, 7, NOW).await;
    }

    #[test]
    fn migrate_turns_the_single_feed_row_into_a_feed_and_keeps_its_events() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE calendar_feed (id INTEGER PRIMARY KEY CHECK (id = 1), url TEXT, last_sync INTEGER, last_error TEXT);
             CREATE TABLE calendar_events (uid TEXT NOT NULL, summary TEXT NOT NULL, start_ms INTEGER NOT NULL, end_ms INTEGER NOT NULL, all_day INTEGER NOT NULL);
             INSERT INTO calendar_feed VALUES (1, 'https://example.com/s.ics', 42, 'old error');
             INSERT INTO calendar_events VALUES ('a', 'Holiday', 1709251200000, 1709337600000, 1);
             INSERT INTO calendar_events VALUES ('b', 'Call', 1709254800000, 1709258400000, 0);",
        )
        .unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap(); // idempotent
        let (name, url, color, vis, sync, error): (String, String, String, bool, i64, String) =
            conn.query_row(
                "SELECT name, url, color, visible, last_sync, last_error FROM calendar_feeds",
                [],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(
            (
                name.as_str(),
                url.as_str(),
                color.as_str(),
                vis,
                sync,
                error.as_str()
            ),
            (
                "Calendar",
                "https://example.com/s.ics",
                "college",
                true,
                42,
                "old error"
            )
        );
        let rows: Vec<(i64, String, Option<String>)> = conn
            .prepare("SELECT feed_id, uid, start_date FROM calendar_events ORDER BY uid")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .flatten()
            .collect();
        assert_eq!(
            rows,
            vec![
                (1, "a".into(), Some("2024-03-01".into())),
                (1, "b".into(), None)
            ]
        );
        let legacy: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'calendar_feed'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(legacy, 0);
    }

    #[test]
    fn migrate_with_an_unconfigured_legacy_row_drops_it_and_orphan_events() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE calendar_feed (id INTEGER PRIMARY KEY CHECK (id = 1), url TEXT, last_sync INTEGER, last_error TEXT);
             CREATE TABLE calendar_events (uid TEXT NOT NULL, summary TEXT NOT NULL, start_ms INTEGER NOT NULL, end_ms INTEGER NOT NULL, all_day INTEGER NOT NULL);
             INSERT INTO calendar_events VALUES ('a', 'x', 1, 2, 0);",
        )
        .unwrap();
        migrate(&conn).unwrap();
        let n: i64 = conn
            .query_row("SELECT (SELECT COUNT(*) FROM calendar_feeds) + (SELECT COUNT(*) FROM calendar_events)", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }
}
