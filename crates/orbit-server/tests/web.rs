use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
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
}
