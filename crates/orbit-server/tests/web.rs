use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use tower::ServiceExt;

async fn get(app: &axum::Router, path: &str) -> (StatusCode, String) {
    let res = app
        .clone()
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

#[tokio::test]
async fn serves_the_web_app_without_shadowing_the_api() {
    let (data, web) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    std::fs::write(web.path().join("index.html"), "<p>index</p>").unwrap();
    std::fs::write(web.path().join("server.html"), "<p>server</p>").unwrap();
    let (app, _) = orbit_server::app(data.path()).unwrap();
    assert_eq!(get(&app, "/").await.0, StatusCode::NOT_FOUND);
    let app = orbit_server::with_web(app, web.path());
    assert_eq!(
        get(&app, "/").await,
        (StatusCode::OK, "<p>index</p>".into())
    );
    assert_eq!(get(&app, "/server.html").await.1, "<p>server</p>");
    assert_eq!(get(&app, "/some/route").await.1, "<p>index</p>");
    assert_eq!(get(&app, "/auth").await.1, r#"{"setup":true}"#);

    let res = app
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let h = res.headers();
    assert_eq!(h[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    assert_eq!(h[header::REFERRER_POLICY], "same-origin");
    let csp = h[header::CONTENT_SECURITY_POLICY].to_str().unwrap();
    assert!(csp.contains("frame-ancestors 'none'") && csp.contains("object-src 'none'"));
}

#[tokio::test]
async fn version_is_public_cacheable_and_has_a_stable_contract() {
    let data = tempfile::tempdir().unwrap();
    let (app, _) = orbit_server::app(data.path()).unwrap();
    let res = app
        .oneshot(Request::get("/version").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get(header::CACHE_CONTROL).unwrap(),
        "public, max-age=300"
    );
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["server"], env!("CARGO_PKG_VERSION"));
    assert_eq!(body["protocol"], orbit_core::PROTOCOL_V);
    assert_eq!(body["min_client_protocol"], orbit_core::MIN_CLIENT_PROTOCOL);
    assert!(body["min_shell_version"].is_string());
    assert!(body["built"].is_string());
    assert!(body["ui"].is_string());
}
