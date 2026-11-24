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

#[tokio::test]
async fn every_calendar_endpoint_needs_a_device() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path(), fake(String::new(), Arc::default())).await;
    for (m, p) in [
        ("GET", "/calendar/events?from=0&to=1"),
        ("GET", "/calendar/feed"),
        ("PUT", "/calendar/feed"),
        ("DELETE", "/calendar/feed"),
    ] {
        let body = (m == "PUT").then(|| json!({ "url": SECRET_URL }));
        let (status, _) = call(&app, m, p, body, false).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{m} {p}");
    }
}

#[tokio::test]
async fn set_feed_fetches_stores_serves_events_and_never_returns_the_url() {
    let dir = tempfile::tempdir().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let app = app(dir.path(), fake(feed_for_today(), calls.clone())).await;

    let (status, body) = call(&app, "GET", "/calendar/feed", None, true).await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        (v["configured"].clone(), v["event_count"].clone()),
        (json!(false), json!(0))
    );

    let (status, body) = call(
        &app,
        "PUT",
        "/calendar/feed",
        Some(json!({ "url": SECRET_URL })),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        !body.contains("TOPSECRET") && !body.contains("me%40x.com"),
        "{body}"
    );
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["url_masked"], "https://calendar.google.com/…");
    assert_eq!(v["event_count"], 4);
    assert_eq!(v["last_error"], Value::Null);
    assert!(v["last_sync_ms"].as_i64().unwrap() > 0);
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let (_, body) = call(&app, "GET", "/calendar/feed", None, true).await;
    assert!(!body.contains("TOPSECRET"));

    let (status, body) = call(
        &app,
        "GET",
        "/calendar/events?from=0&to=99999999999999",
        None,
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let ev: Vec<Value> = serde_json::from_str(&body).unwrap();
    assert_eq!(ev.len(), 4);
    assert!(
        ev.iter()
            .any(|e| e["summary"] == "Holiday" && e["all_day"] == true)
    );
    let (_, body) = call(&app, "GET", "/calendar/events?from=0&to=1", None, true).await;
    assert_eq!(body, "[]");

    let (_, body) = call(&app, "DELETE", "/calendar/feed", None, true).await;
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(
        (v["configured"].clone(), v["event_count"].clone()),
        (json!(false), json!(0))
    );
}

#[tokio::test]
async fn set_feed_rejects_unsafe_urls_without_fetching_or_storing() {
    let dir = tempfile::tempdir().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let app = app(dir.path(), fake(feed_for_today(), calls.clone())).await;
    for u in [
        "http://calendar.google.com/a.ics",
        "https://127.0.0.1/a.ics",
        "https://[::1]/a.ics",
    ] {
        let (status, body) = call(
            &app,
            "PUT",
            "/calendar/feed",
            Some(json!({ "url": u })),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{u}");
        assert!(!body.contains(u));
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let (_, body) = call(&app, "GET", "/calendar/feed", None, true).await;
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap()["configured"],
        false
    );
}

#[tokio::test]
async fn fetch_failure_shows_in_health_not_as_an_http_error() {
    let dir = tempfile::tempdir().unwrap();
    let failing: Fetcher =
        Arc::new(|_| Box::pin(async { Err("feed returned HTTP 404".to_string()) }));
    let app = app(dir.path(), failing).await;
    let (status, body) = call(
        &app,
        "PUT",
        "/calendar/feed",
        Some(json!({ "url": SECRET_URL })),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["last_error"], "feed returned HTTP 404");
    assert_eq!(v["last_sync_ms"], Value::Null);
}
