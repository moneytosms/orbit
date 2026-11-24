//! Read-only Google Calendar via a secret iCal feed URL: storage, SSRF-safe fetch, poll, endpoints.
//!
//! The URL is a secret: it is never logged, never put in an error, and clients only see
//! `scheme://host/…`. Events live in `calendar_events`, outside the synced row store.

use std::{
    future::Future,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use reqwest::{Url, redirect::Policy};
use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{St, auth, ical, lock, now_ms};

const DAY_MS: i64 = 86_400_000;
/// How often the poll job refreshes a configured feed.
pub const POLL_INTERVAL: Duration = Duration::from_secs(30 * 60);
/// Days before now and after now that occurrences are expanded and stored.
const PAST_DAYS: i64 = 31;
const FUTURE_DAYS: i64 = 400;
const MAX_BYTES: usize = 5 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(15);
const MAX_REDIRECTS: usize = 3;
const MAX_ROWS: i64 = 5_000;

/// Fetches a feed URL to bytes. Tests swap in a fake; the real one is `fetch_feed`.
pub type Fetcher = Arc<
    dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + Send>> + Send + Sync,
>;

pub fn real_fetcher() -> Fetcher {
    Arc::new(|url| Box::pin(fetch_feed(url)))
}

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS calendar_feed (id INTEGER PRIMARY KEY CHECK (id = 1), url TEXT, last_sync INTEGER, last_error TEXT);
         CREATE TABLE IF NOT EXISTS calendar_events (uid TEXT NOT NULL, summary TEXT NOT NULL, start_ms INTEGER NOT NULL, end_ms INTEGER NOT NULL, all_day INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS calendar_events_start ON calendar_events (start_ms);",
    )
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

fn feed_url(conn: &Connection) -> rusqlite::Result<Option<String>> {
    Ok(conn
        .query_row("SELECT url FROM calendar_feed WHERE id = 1", [], |r| {
            r.get::<_, Option<String>>(0)
        })
        .optional()?
        .flatten())
}

fn record_error(db: &Mutex<Connection>, msg: &str) {
    let _ = lock(db).execute(
        "UPDATE calendar_feed SET last_error = ?1 WHERE id = 1",
        [msg],
    );
}

/// Fetches the configured feed and replaces the stored events. On failure the old events stay and
/// `last_error` is set; `last_sync` only moves on success. No URL configured: does nothing.
pub async fn refresh(db: &Mutex<Connection>, fetch: &Fetcher, now: i64) {
    let Ok(Some(url)) = feed_url(&lock(db)) else {
        return;
    };
    let body = match fetch(url).await {
        Ok(b) => b,
        Err(e) => return record_error(db, &e),
    };
    let Ok(text) = String::from_utf8(body) else {
        return record_error(db, "feed is not UTF-8 text");
    };
    if !text.contains("BEGIN:VCALENDAR") {
        return record_error(db, "response is not an iCalendar feed");
    }
    let events = ical::parse_feed(&text, now - PAST_DAYS * DAY_MS, now + FUTURE_DAYS * DAY_MS);
    let mut conn = lock(db);
    let res = (|| {
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM calendar_events", [])?;
        for e in &events {
            tx.execute(
                "INSERT INTO calendar_events (uid, summary, start_ms, end_ms, all_day) VALUES (?1, ?2, ?3, ?4, ?5)",
                (&e.uid, &e.summary, e.start_ms, e.end_ms, e.all_day),
            )?;
        }
        tx.execute(
            "UPDATE calendar_feed SET last_sync = ?1, last_error = NULL WHERE id = 1",
            [now],
        )?;
        tx.commit()
    })();
    if res.is_err() {
        drop(conn);
        record_error(db, "could not store events");
    }
}

/// Refreshes the feed now and then every `POLL_INTERVAL`, for the life of the server.
pub async fn poll(st: St) {
    loop {
        refresh(&st.db, &st.fetcher, now_ms()).await;
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

// ---- endpoints ----

fn health(conn: &Connection) -> rusqlite::Result<Value> {
    let (url, last_sync, last_error) = conn
        .query_row(
            "SELECT url, last_sync, last_error FROM calendar_feed WHERE id = 1",
            [],
            |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<i64>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()?
        .unwrap_or_default();
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM calendar_events", [], |r| r.get(0))?;
    Ok(json!({
        "configured": url.is_some(),
        "url_masked": url.as_deref().map(mask),
        "last_sync_ms": last_sync,
        "last_error": last_error,
        "event_count": count,
    }))
}

fn internal(e: rusqlite::Error) -> StatusCode {
    eprintln!("calendar: {e}");
    StatusCode::INTERNAL_SERVER_ERROR
}

pub(crate) async fn feed(State(st): State<St>, _: auth::Device) -> Result<Json<Value>, StatusCode> {
    health(&lock(&st.db)).map(Json).map_err(internal)
}

#[derive(Deserialize)]
pub(crate) struct SetFeed {
    url: String,
}

/// Stores the URL, refreshes once, and returns the health (a failed fetch shows up there, not as an error).
pub(crate) async fn set_feed(
    State(st): State<St>,
    _: auth::Device,
    Json(body): Json<SetFeed>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let url =
        check_url(&body.url).map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))))?;
    let internal = |e| (internal(e), Json(json!({ "error": "storage failed" })));
    {
        let mut conn = lock(&st.db);
        let tx = conn.transaction().map_err(internal)?;
        tx.execute("DELETE FROM calendar_events", [])
            .map_err(internal)?;
        tx.execute(
            "INSERT INTO calendar_feed (id, url, last_sync, last_error) VALUES (1, ?1, NULL, NULL)
             ON CONFLICT (id) DO UPDATE SET url = ?1, last_sync = NULL, last_error = NULL",
            [url.as_str()],
        )
        .map_err(internal)?;
        tx.commit().map_err(internal)?;
    }
    refresh(&st.db, &st.fetcher, now_ms()).await;
    health(&lock(&st.db)).map(Json).map_err(internal)
}

pub(crate) async fn clear_feed(
    State(st): State<St>,
    _: auth::Device,
) -> Result<Json<Value>, StatusCode> {
    let conn = lock(&st.db);
    conn.execute_batch("DELETE FROM calendar_events; DELETE FROM calendar_feed;")
        .map_err(internal)?;
    health(&conn).map(Json).map_err(internal)
}

#[derive(Deserialize)]
pub(crate) struct Window {
    from: i64,
    to: i64,
}

/// Stored events overlapping `[from, to)` (epoch ms), oldest first.
pub(crate) async fn events(
    State(st): State<St>,
    _: auth::Device,
    Query(w): Query<Window>,
) -> Result<Json<Vec<ical::Event>>, StatusCode> {
    let conn = lock(&st.db);
    let mut stmt = conn
        .prepare(
            "SELECT uid, summary, start_ms, end_ms, all_day FROM calendar_events
             WHERE start_ms < ?2 AND MAX(end_ms, start_ms + 1) > ?1 ORDER BY start_ms, uid LIMIT ?3",
        )
        .map_err(internal)?;
    let rows = stmt
        .query_map((w.from, w.to, MAX_ROWS), |r| {
            Ok(ical::Event {
                uid: r.get(0)?,
                summary: r.get(1)?,
                start_ms: r.get(2)?,
                end_ms: r.get(3)?,
                all_day: r.get(4)?,
            })
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
        conn.execute(
            "INSERT INTO calendar_feed (id, url) VALUES (1, 'https://example.com/secret.ics')",
            [],
        )
        .unwrap();
        Mutex::new(conn)
    }

    const FEED: &str = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:a\r\nDTSTART:20240101T100000Z\r\nRRULE:FREQ=DAILY;COUNT=3\r\nSUMMARY:S\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    const NOW: i64 = 1_704_067_200_000; // 2024-01-01T00:00:00Z

    fn state(db: &Mutex<Connection>) -> (i64, Option<i64>, Option<String>) {
        let c = lock(db);
        let n = c
            .query_row("SELECT COUNT(*) FROM calendar_events", [], |r| r.get(0))
            .unwrap();
        let (s, e) = c
            .query_row("SELECT last_sync, last_error FROM calendar_feed", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        (n, s, e)
    }

    #[tokio::test]
    async fn refresh_stores_events_and_records_last_sync() {
        let db = db();
        refresh(&db, &fake(Ok(FEED)), NOW).await;
        assert_eq!(state(&db), (3, Some(NOW), None));
    }

    #[tokio::test]
    async fn failed_refresh_keeps_old_events_and_records_error_without_moving_last_sync() {
        let db = db();
        refresh(&db, &fake(Ok(FEED)), NOW).await;
        refresh(&db, &fake(Err("feed returned HTTP 500")), NOW + 1).await;
        assert_eq!(
            state(&db),
            (3, Some(NOW), Some("feed returned HTTP 500".into()))
        );
        refresh(&db, &fake(Ok("<html>nope</html>")), NOW + 2).await;
        assert_eq!(
            state(&db).2.as_deref(),
            Some("response is not an iCalendar feed")
        );
        refresh(&db, &fake(Ok(FEED)), NOW + 3).await;
        assert_eq!(state(&db), (3, Some(NOW + 3), None));
    }

    #[tokio::test]
    async fn refresh_without_a_url_never_fetches() {
        let db = db();
        lock(&db).execute("DELETE FROM calendar_feed", []).unwrap();
        let fetch: Fetcher = Arc::new(|_| panic!("fetched without a URL"));
        refresh(&db, &fetch, NOW).await;
    }
}
