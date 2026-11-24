use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{HeaderMap, Method, Request, StatusCode, header},
};
use serde_json::{Value, json};
use std::net::SocketAddr;
use tower::ServiceExt;

const PW: &str = "correct horse battery";

/// One request from 127.0.0.1 with an optional Bearer token.
async fn call(
    app: &Router,
    method: Method,
    path: &str,
    body: Value,
    token: Option<&str>,
) -> (StatusCode, HeaderMap, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let mut req = req.body(Body::from(body.to_string())).unwrap();
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1))));
    let res = app.clone().oneshot(req).await.unwrap();
    let (status, headers) = (res.status(), res.headers().clone());
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, headers, body)
}

async fn post(app: &Router, path: &str, body: Value, token: Option<&str>) -> (StatusCode, Value) {
    let (s, _, b) = call(app, Method::POST, path, body, token).await;
    (s, b)
}

/// A fresh server with its user set up; returns the app and the first Device's token.
async fn setup() -> (Router, String, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let (app, code) = orbit_server::app(dir.path()).unwrap();
    let (s, res) = post(
        &app,
        "/auth/setup",
        json!({ "code": code.unwrap(), "password": PW, "name": "Laptop", "platform": "cli" }),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{res}");
    (app, res["token"].as_str().unwrap().into(), dir)
}

fn sync_body() -> Value {
    json!({ "v": 1, "commands": [] })
}

#[tokio::test]
async fn first_run_needs_the_setup_code_and_creates_one_user() {
    let dir = tempfile::tempdir().unwrap();
    let (app, code) = orbit_server::app(dir.path()).unwrap();
    let code = code.expect("a fresh server prints a setup code");
    let (_, _, state) = call(&app, Method::GET, "/auth", Value::Null, None).await;
    assert_eq!(state, json!({ "setup": true }));

    let wrong = json!({ "code": "nope", "password": PW, "name": "Laptop", "platform": "cli" });
    assert_eq!(
        post(&app, "/auth/setup", wrong, None).await.0,
        StatusCode::FORBIDDEN
    );

    let ok = json!({ "code": code, "password": PW, "name": "Laptop", "platform": "cli" });
    let (s, res) = post(&app, "/auth/setup", ok.clone(), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(res["token"].as_str().unwrap().len(), 64);
    assert!(res["device"].is_string());
    // The code is spent; a restart prints none.
    assert_eq!(
        post(&app, "/auth/setup", ok, None).await.0,
        StatusCode::FORBIDDEN
    );
    let (_, _, state) = call(&app, Method::GET, "/auth", Value::Null, None).await;
    assert_eq!(state, json!({ "setup": false }));
    assert!(orbit_server::app(dir.path()).unwrap().1.is_none());
}

#[tokio::test]
async fn sync_needs_a_device_token() {
    let (app, token, _dir) = setup().await;
    assert_eq!(
        post(&app, "/sync", sync_body(), None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        post(&app, "/sync", sync_body(), Some("f".repeat(64).as_str()))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        post(&app, "/sync", sync_body(), Some(&token)).await.0,
        StatusCode::OK
    );
}

fn login(totp: Option<String>) -> Value {
    json!({ "password": PW, "totp": totp, "name": "Phone", "platform": "android" })
}

fn code_for(secret: &str) -> String {
    totp_rs::Builder::new()
        .with_secret(totp_rs::Secret::try_from_base32(secret).unwrap())
        .build()
        .unwrap()
        .generate_current()
        .to_string()
}

#[tokio::test]
async fn login_creates_a_device_and_asks_for_totp_once_enrolled() {
    let (app, token, _dir) = setup().await;
    let (s, res) = post(&app, "/auth/login", login(None), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_ne!(res["token"], Value::Null);

    let bad = json!({ "password": "wrong password", "name": "Phone", "platform": "android" });
    assert_eq!(
        post(&app, "/auth/login", bad, None).await.0,
        StatusCode::UNAUTHORIZED
    );

    // Enrollment only counts once a code from the new secret is confirmed.
    let (s, _) = post(
        &app,
        "/auth/totp",
        json!({ "password": "wrong password" }),
        Some(&token),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (_, enroll) = post(&app, "/auth/totp", json!({ "password": PW }), Some(&token)).await;
    let secret = enroll["secret"].as_str().unwrap();
    assert!(
        enroll["url"]
            .as_str()
            .unwrap()
            .starts_with("otpauth://totp/")
    );
    assert_eq!(
        post(&app, "/auth/login", login(None), None).await.0,
        StatusCode::OK
    );
    let (s, _) = post(
        &app,
        "/auth/totp/confirm",
        json!({ "code": "000000x" }),
        Some(&token),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = post(
        &app,
        "/auth/totp/confirm",
        json!({ "code": code_for(secret) }),
        Some(&token),
    )
    .await;
    assert_eq!(s, StatusCode::NO_CONTENT);

    let (s, res) = post(&app, "/auth/login", login(None), None).await;
    assert_eq!(
        (s, res),
        (StatusCode::UNAUTHORIZED, json!({ "error": "totp" }))
    );
    let (s, _) = post(&app, "/auth/login", login(Some("123456x".into())), None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = post(&app, "/auth/login", login(Some(code_for(secret))), None).await;
    assert_eq!(s, StatusCode::OK);
}

#[tokio::test]
async fn web_logins_get_an_http_only_cookie_and_no_token_in_the_body() {
    let (app, _, _dir) = setup().await;
    let body = json!({ "password": PW, "name": "Firefox", "platform": "web" });
    let (s, headers, res) = call(&app, Method::POST, "/auth/login", body, None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(res["token"], Value::Null);
    let cookie = headers[header::SET_COOKIE].to_str().unwrap();
    assert!(
        cookie.contains("HttpOnly") && cookie.contains("SameSite=Strict"),
        "{cookie}"
    );

    let pair = cookie.split(';').next().unwrap();
    let mut req = Request::post("/sync")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, pair)
        .body(Body::from(sync_body().to_string()))
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1))));
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Thirty-day sliding session: the login cookie and every cookie-authenticated reply carry it.
    assert!(cookie.contains("Max-Age=2592000"), "{cookie}");
    assert_eq!(res.headers()[header::SET_COOKIE].to_str().unwrap(), cookie);
}

#[tokio::test]
async fn bearer_requests_do_not_get_a_cookie() {
    let (app, token, _dir) = setup().await;
    let (s, headers, _) = call(&app, Method::POST, "/sync", sync_body(), Some(&token)).await;
    assert_eq!(s, StatusCode::OK);
    assert!(!headers.contains_key(header::SET_COOKIE));
}

#[tokio::test]
async fn devices_are_listed_revoked_and_created_as_script_tokens() {
    let (app, laptop, _dir) = setup().await;
    let (_, phone) = post(&app, "/auth/login", login(None), None).await;
    let (s, script) = post(
        &app,
        "/devices",
        json!({ "name": "Backup cron" }),
        Some(&laptop),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let script_token = script["token"].as_str().unwrap();
    assert_eq!(
        post(&app, "/sync", sync_body(), Some(script_token)).await.0,
        StatusCode::OK
    );

    let (_, _, list) = call(&app, Method::GET, "/devices", Value::Null, Some(&laptop)).await;
    let names: Vec<_> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            (
                d["name"].clone(),
                d["platform"].clone(),
                d["current"].clone(),
            )
        })
        .collect();
    assert_eq!(
        names,
        [
            (json!("Laptop"), json!("cli"), json!(true)),
            (json!("Phone"), json!("android"), json!(false)),
            (json!("Backup cron"), json!("script"), json!(false)),
        ]
    );
    assert!(list[0]["last_seen"].as_i64().unwrap() >= list[0]["created"].as_i64().unwrap());

    let path = format!("/devices/{}", phone["device"].as_str().unwrap());
    let (s, _, _) = call(&app, Method::DELETE, &path, Value::Null, Some(&laptop)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let phone_token = phone["token"].as_str().unwrap();
    assert_eq!(
        post(&app, "/sync", sync_body(), Some(phone_token)).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn five_failed_logins_a_minute_lock_the_ip_out() {
    let (app, _, _dir) = setup().await;
    let bad = json!({ "password": "wrong password", "name": "Phone", "platform": "android" });
    for _ in 0..5 {
        assert_eq!(
            post(&app, "/auth/login", bad.clone(), None).await.0,
            StatusCode::UNAUTHORIZED
        );
    }
    // Even the right password waits out the lockout.
    assert_eq!(
        post(&app, "/auth/login", login(None), None).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn changing_the_password_can_sign_out_other_devices() {
    let (app, laptop, _dir) = setup().await;
    let (_, phone) = post(&app, "/auth/login", login(None), None).await;
    let phone = phone["token"].as_str().unwrap();
    let change = |current: &str, sign_out_others| json!({ "current": current, "new": "a new long password", "sign_out_others": sign_out_others });

    let (s, _) = post(
        &app,
        "/auth/password",
        change("wrong password", false),
        Some(&laptop),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, _) = post(&app, "/auth/password", change(PW, true), Some(&laptop)).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    assert_eq!(
        post(&app, "/sync", sync_body(), Some(&laptop)).await.0,
        StatusCode::OK
    );
    assert_eq!(
        post(&app, "/sync", sync_body(), Some(phone)).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        post(&app, "/auth/login", login(None), None).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn reset_password_sets_a_new_one_and_revokes_every_device() {
    let (app, laptop, dir) = setup().await;
    orbit_server::reset_password(dir.path(), "reset long password").unwrap();
    assert_eq!(
        post(&app, "/sync", sync_body(), Some(&laptop)).await.0,
        StatusCode::UNAUTHORIZED
    );
    let body = json!({ "password": "reset long password", "name": "Laptop", "platform": "cli" });
    assert_eq!(
        post(&app, "/auth/login", body, None).await.0,
        StatusCode::OK
    );
}

/// One request from `peer` with extra headers.
async fn from(
    app: &Router,
    peer: [u8; 4],
    path: &str,
    body: Value,
    headers: &[(&str, &str)],
) -> StatusCode {
    let mut req = Request::post(path).header(header::CONTENT_TYPE, "application/json");
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let mut req = req.body(Body::from(body.to_string())).unwrap();
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from((peer, 1))));
    app.clone().oneshot(req).await.unwrap().status()
}

fn bad_login() -> Value {
    json!({ "password": "wrong password", "name": "Phone", "platform": "android" })
}

#[tokio::test]
async fn forwarded_ip_headers_are_ignored_from_non_loopback_peers() {
    let (app, _, _dir) = setup().await;
    let lan = [10, 0, 0, 5];
    // If the header were trusted these would be five different buckets and never lock.
    for n in 1..=5 {
        let ip = format!("9.9.9.{n}");
        let s = from(
            &app,
            lan,
            "/auth/login",
            bad_login(),
            &[("cf-connecting-ip", &ip)],
        )
        .await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
    }
    let s = from(
        &app,
        lan,
        "/auth/login",
        login(None),
        &[("cf-connecting-ip", "8.8.8.8")],
    )
    .await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn loopback_peers_get_one_lockout_bucket_per_forwarded_client() {
    let (app, _, _dir) = setup().await;
    let lo = [127, 0, 0, 1];
    for _ in 0..5 {
        let s = from(
            &app,
            lo,
            "/auth/login",
            bad_login(),
            &[("cf-connecting-ip", "1.1.1.1")],
        )
        .await;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
    }
    let locked = from(
        &app,
        lo,
        "/auth/login",
        login(None),
        &[("cf-connecting-ip", "1.1.1.1")],
    )
    .await;
    assert_eq!(locked, StatusCode::TOO_MANY_REQUESTS);
    // Another client behind the same tunnel, via CF-Connecting-IP or X-Forwarded-For, is not locked.
    let other = from(
        &app,
        lo,
        "/auth/login",
        login(None),
        &[("cf-connecting-ip", "2.2.2.2")],
    )
    .await;
    assert_eq!(other, StatusCode::OK);
    let xff = from(
        &app,
        lo,
        "/auth/login",
        login(None),
        &[("x-forwarded-for", "3.3.3.3, 127.0.0.1")],
    )
    .await;
    assert_eq!(xff, StatusCode::OK);
    // No header: falls back to the peer address, which has its own bucket.
    for _ in 0..5 {
        assert_eq!(
            from(&app, lo, "/auth/login", bad_login(), &[]).await,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        from(&app, lo, "/auth/login", login(None), &[]).await,
        StatusCode::TOO_MANY_REQUESTS
    );
    // An unparsable header also falls back to the peer address.
    let junk = from(
        &app,
        lo,
        "/auth/login",
        login(None),
        &[("cf-connecting-ip", "nope")],
    )
    .await;
    assert_eq!(junk, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn bearer_wins_over_the_cookie_and_other_schemes_are_rejected() {
    let (app, token, _dir) = setup().await;
    let cookie = format!("orbit={token}");
    let bearer = format!("Bearer {token}");
    let basic = format!("Basic {token}");
    let lower = format!("bearer {token}");
    async fn sync(app: &Router, h: &[(&str, &str)]) -> StatusCode {
        from(app, [127, 0, 0, 1], "/sync", sync_body(), h).await
    }
    let auth = "authorization";
    assert_eq!(sync(&app, &[(auth, &bearer)]).await, StatusCode::OK);
    assert_eq!(sync(&app, &[("cookie", &cookie)]).await, StatusCode::OK);
    // Not a `Bearer ` scheme: ignored, so no credential unless a cookie is present.
    assert_eq!(
        sync(&app, &[(auth, &basic)]).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        sync(&app, &[(auth, &lower)]).await,
        StatusCode::UNAUTHORIZED
    );
    // Both present: the Bearer token is used and the cookie is never consulted, either way round.
    let both = [(auth, bearer.as_str()), ("cookie", "orbit=bogus")];
    assert_eq!(sync(&app, &both).await, StatusCode::OK);
    let both = [(auth, "Bearer bogus"), ("cookie", cookie.as_str())];
    assert_eq!(sync(&app, &both).await, StatusCode::UNAUTHORIZED);
    // A non-Bearer scheme falls through to the cookie.
    let both = [(auth, "Basic xxx"), ("cookie", cookie.as_str())];
    assert_eq!(sync(&app, &both).await, StatusCode::OK);
}

#[tokio::test]
async fn the_web_cookie_is_secure_only_when_the_flag_is_set() {
    for secure in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let (app, code) = orbit_server::app_secure_cookie(dir.path(), secure).unwrap();
        let setup =
            json!({ "code": code.unwrap(), "password": PW, "name": "Laptop", "platform": "cli" });
        assert_eq!(
            post(&app, "/auth/setup", setup, None).await.0,
            StatusCode::OK
        );
        let body = json!({ "password": PW, "name": "Firefox", "platform": "web" });
        let (_, headers, _) = call(&app, Method::POST, "/auth/login", body, None).await;
        let cookie = headers[header::SET_COOKIE].to_str().unwrap();
        assert_eq!(cookie.contains("; Secure"), secure, "{cookie}");
    }
}
