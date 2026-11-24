use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use orbit_server::Fetcher;
use serde_json::{Value, json};
use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tower::ServiceExt;

const SECRET_URL: &str =
    "https://calendar.google.com/calendar/ical/me%40x.com/private-TOPSECRET/basic.ics";

async fn app(dir: &std::path::Path, fetcher: Fetcher) -> (Router, String) {
    let (app, code) = orbit_server::app_with_fetcher(dir, fetcher).unwrap();
    let body = json!({ "code": code.unwrap(), "password": "a long password", "name": "Test", "platform": "cli" });
    let mut req = Request::post("/auth/setup")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1))));
    let res = app.clone().oneshot(req).await.unwrap();
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let res: Value = serde_json::from_slice(&bytes).unwrap();
    (app, res["token"].as_str().unwrap().into())
}

async fn call(
    (app, token): &(Router, String),
    method: &str,
    path: &str,
    body: Option<Value>,
    auth: bool,
) -> (StatusCode, String) {
    let mut req = Request::builder().method(method).uri(path);
    if auth {
        req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = match body {
        Some(b) => {
            req = req.header(header::CONTENT_TYPE, "application/json");
            Body::from(b.to_string())
        }
        None => Body::empty(),
    };
    let res = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

fn feed_for_today() -> String {
    // An all-day event today (UTC) and a daily 3-count timed event starting today.
    let day = today();
    format!(
        "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:a\r\nDTSTART;VALUE=DATE:{day}\r\nSUMMARY:Holiday\r\nEND:VEVENT\r\n\
         BEGIN:VEVENT\r\nUID:b\r\nDTSTART:{day}T100000Z\r\nDTEND:{day}T110000Z\r\nRRULE:FREQ=DAILY;COUNT=3\r\nSUMMARY:Standup\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n"
    )
}

fn today() -> String {
    chrono::DateTime::from_timestamp(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        0,
    )
    .unwrap()
    .format("%Y%m%d")
    .to_string()
}

fn fake(body: String, calls: Arc<AtomicUsize>) -> Fetcher {
    Arc::new(move |url| {
        assert!(url.starts_with("https://"));
        calls.fetch_add(1, Ordering::SeqCst);
        let body = body.clone();
        Box::pin(async move { Ok(body.into_bytes()) })
    })
}

fn json_of(body: &str) -> Value {
    serde_json::from_str(body).unwrap()
}

async fn add(app: &(Router, String), name: &str, url: &str) -> (StatusCode, String) {
    call(
        app,
        "POST",
        "/calendar/feeds",
        Some(json!({ "name": name, "url": url })),
        true,
    )
    .await
}

#[tokio::test]
async fn every_calendar_endpoint_needs_a_device() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path(), fake(String::new(), Arc::default())).await;
    for (m, p) in [
        ("GET", "/calendar/events?from=0&to=1"),
        ("GET", "/calendar/feeds"),
        ("POST", "/calendar/feeds"),
        ("PATCH", "/calendar/feeds/1"),
        ("DELETE", "/calendar/feeds/1"),
    ] {
        let body = matches!(m, "POST" | "PATCH")
            .then(|| json!({ "name": "x", "url": SECRET_URL, "visible": false }));
        let (status, _) = call(&app, m, p, body, false).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{m} {p}");
    }
}

#[tokio::test]
async fn feeds_are_added_listed_edited_and_removed_and_the_url_never_comes_back() {
    let dir = tempfile::tempdir().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let app = app(dir.path(), fake(feed_for_today(), calls.clone())).await;

    let (_, body) = call(&app, "GET", "/calendar/feeds", None, true).await;
    assert_eq!(body, "[]");

    let (status, body) = add(&app, "Work", SECRET_URL).await;
    assert_eq!(status, StatusCode::OK);
    let secret = |b: &str| b.contains("TOPSECRET") || b.contains("me%40x.com");
    assert!(!secret(&body), "{body}");
    let v = json_of(&body);
    assert_eq!(v["name"], "Work");
    assert_eq!(v["url_masked"], "https://calendar.google.com/…");
    assert_eq!(v["visible"], true);
    assert_eq!(v["event_count"], 4);
    assert_eq!(v["last_error"], Value::Null);
    assert!(v["last_sync_ms"].as_i64().unwrap() > 0);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let id = v["id"].as_i64().unwrap();

    let (_, body) = call(&app, "GET", "/calendar/feeds", None, true).await;
    assert!(!secret(&body));
    assert_eq!(json_of(&body).as_array().unwrap().len(), 1);

    let (status, body) = call(
        &app,
        "PATCH",
        &format!("/calendar/feeds/{id}"),
        Some(json!({ "name": "Job", "color": "career" })),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let v = json_of(&body);
    assert_eq!(
        (v["name"].clone(), v["color"].clone()),
        (json!("Job"), json!("career"))
    );
    assert!(!secret(&body));

    let (_, body) = call(
        &app,
        "GET",
        "/calendar/events?from=0&to=99999999999999",
        None,
        true,
    )
    .await;
    assert!(!secret(&body));
    let ev = json_of(&body);
    let ev = ev.as_array().unwrap();
    assert_eq!(ev.len(), 4);
    assert!(
        ev.iter()
            .all(|e| e["feed_id"] == id && e["feed_name"] == "Job" && e["color"] == "career")
    );
    let holiday = ev.iter().find(|e| e["summary"] == "Holiday").unwrap();
    assert_eq!(holiday["all_day"], true);
    assert_eq!(holiday["start_date"].as_str().unwrap().len(), 10);
    let standup = ev.iter().find(|e| e["summary"] == "Standup").unwrap();
    assert_eq!(standup["start_date"], Value::Null);
    let (_, body) = call(&app, "GET", "/calendar/events?from=0&to=1", None, true).await;
    assert_eq!(body, "[]");

    let (status, _) = call(&app, "DELETE", &format!("/calendar/feeds/{id}"), None, true).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, body) = call(
        &app,
        "GET",
        "/calendar/events?from=0&to=99999999999999",
        None,
        true,
    )
    .await;
    assert_eq!(body, "[]");
    let (status, _) = call(&app, "DELETE", &format!("/calendar/feeds/{id}"), None, true).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(
        &app,
        "PATCH",
        "/calendar/feeds/999",
        Some(json!({ "visible": false })),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn hidden_feeds_contribute_no_events() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path(), fake(feed_for_today(), Arc::default())).await;
    let a = json_of(&add(&app, "A", SECRET_URL).await.1)["id"]
        .as_i64()
        .unwrap();
    let b = json_of(
        &add(&app, "B", "https://example.com/other-secret.ics")
            .await
            .1,
    )["id"]
        .as_i64()
        .unwrap();
    let count = |app: &(Router, String)| {
        let app = app.clone();
        async move {
            let (_, body) = call(
                &app,
                "GET",
                "/calendar/events?from=0&to=99999999999999",
                None,
                true,
            )
            .await;
            let v = json_of(&body);
            let mut ids: Vec<i64> = v
                .as_array()
                .unwrap()
                .iter()
                .map(|e| e["feed_id"].as_i64().unwrap())
                .collect();
            ids.sort();
            ids.dedup();
            (v.as_array().unwrap().len(), ids)
        }
    };
    assert_eq!(count(&app).await, (8, vec![a, b]));
    call(
        &app,
        "PATCH",
        &format!("/calendar/feeds/{a}"),
        Some(json!({ "visible": false })),
        true,
    )
    .await;
    assert_eq!(count(&app).await, (4, vec![b]));
    // Hiding keeps health and events stored; showing again brings them back.
    let (_, body) = call(&app, "GET", "/calendar/feeds", None, true).await;
    let feeds = json_of(&body);
    assert_eq!(feeds[0]["visible"], false);
    assert_eq!(feeds[0]["event_count"], 4);
    call(
        &app,
        "PATCH",
        &format!("/calendar/feeds/{a}"),
        Some(json!({ "visible": true })),
        true,
    )
    .await;
    assert_eq!(count(&app).await.0, 8);
}

#[tokio::test]
async fn create_rejects_unsafe_urls_bad_input_and_the_eleventh_feed_without_fetching() {
    let dir = tempfile::tempdir().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let app = app(dir.path(), fake(feed_for_today(), calls.clone())).await;
    for u in [
        "http://calendar.google.com/a.ics",
        "https://127.0.0.1/a.ics",
        "https://[::1]/a.ics",
    ] {
        let (status, body) = add(&app, "X", u).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{u}");
        assert!(!body.contains(u));
    }
    for (name, color) in [("", "college"), ("   ", "college"), ("ok", "hotpink")] {
        let (status, _) = call(
            &app,
            "POST",
            "/calendar/feeds",
            Some(json!({ "name": name, "url": SECRET_URL, "color": color })),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{name:?} {color}");
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        call(&app, "GET", "/calendar/feeds", None, true).await.1,
        "[]"
    );

    for i in 0..10 {
        let (status, _) = add(&app, "F", &format!("https://example.com/{i}.ics")).await;
        assert_eq!(status, StatusCode::OK);
    }
    let (status, body) = add(&app, "Extra", "https://example.com/extra.ics").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("at most 10"));
    assert_eq!(calls.load(Ordering::SeqCst), 10);
}

#[tokio::test]
async fn fetch_failure_shows_in_health_not_as_an_http_error() {
    let dir = tempfile::tempdir().unwrap();
    let failing: Fetcher =
        Arc::new(|_| Box::pin(async { Err("feed returned HTTP 404".to_string()) }));
    let app = app(dir.path(), failing).await;
    let (status, body) = add(&app, "Gone", SECRET_URL).await;
    assert_eq!(status, StatusCode::OK);
    let v = json_of(&body);
    assert_eq!(v["last_error"], "feed returned HTTP 404");
    assert_eq!(v["last_sync_ms"], Value::Null);
}
