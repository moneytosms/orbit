//! The Todoist import against a local fake Todoist API: no network, no real token.
use axum::{
    Json, Router,
    body::{Body, to_bytes},
    extract::{ConnectInfo, State},
    http::Request,
    http::{HeaderMap, StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tower::ServiceExt;

const TOKEN: &str = "fake-todoist-token-123";
const PNG: &[u8] = b"\x89PNG fake image bytes";

/// What the fake saw: `(path?query, bearer token)` per request.
#[derive(Default)]
struct Seen(Mutex<Vec<(String, Option<String>)>>);

fn q(uri: &Uri) -> HashMap<String, String> {
    uri.query()
        .unwrap_or("")
        .split('&')
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.into(), v.into()))
        .collect()
}

async fn fake(State(seen): State<Arc<Seen>>, uri: Uri, headers: HeaderMap) -> Response {
    let auth = headers
        .get(header::AUTHORIZATION)
        .map(|v| v.to_str().unwrap().to_string());
    let hits = {
        let mut s = seen.0.lock().unwrap();
        s.push((uri.to_string(), auth));
        s.iter().filter(|(p, _)| p.starts_with("/tasks")).count()
    };
    let q = q(&uri);
    let host = headers
        .get(header::HOST)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let task = |id: &str, project: &str, content: &str, extra: Value| {
        let mut t = json!({ "id": id, "project_id": project, "content": content, "priority": 1,
                            "child_order": 1, "labels": [], "checked": false });
        t.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        t
    };
    match uri.path() {
        "/projects" => Json(json!({ "results": [
            { "id": "pi", "name": "Inbox", "inbox_project": true, "child_order": 0 },
            { "id": "pw", "name": "Work", "color": "blue", "child_order": 1 },
            { "id": "pweb", "name": "Website", "color": "teal", "parent_id": "pw", "child_order": 1 },
            { "id": "plaunch", "name": "Launch", "parent_id": "pweb", "child_order": 1 },
        ], "next_cursor": null }))
        .into_response(),
        "/sections" => Json(json!({ "results": [
            { "id": "s1", "project_id": "pweb", "name": "Backlog", "section_order": 1 },
            { "id": "s2", "project_id": "pi", "name": "Sorted", "section_order": 1 },
        ] }))
        .into_response(),
        "/labels" => {
            Json(json!({ "results": [{ "id": "l1", "name": "Errands", "color": "berry_red" }] }))
                .into_response()
        }
        "/tasks" => {
            // The first request meets a rate limit; later pages come by cursor.
            if hits == 1 {
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    [("retry-after", "0")],
                    "slow down",
                )
                    .into_response();
            }
            if q.get("cursor").map(String::as_str) == Some("page2") {
                return Json(json!({ "results": [
                    task("t4", "pweb", "Publish", json!({ "parent_id": "t2", "child_order": 2 })),
                    task("t5", "plaunch", "Standup", json!({
                        "due": { "date": "2026-03-04", "string": "every 2nd tuesday of the month", "is_recurring": true } })),
                    task("t6", "pw", "Report", json!({ "priority": 3,
                        "due": { "date": "2026-03-05", "string": "every! 2 weeks", "is_recurring": true } })),
                ], "next_cursor": null })).into_response();
            }
            Json(json!({ "results": [
                task("t1", "pi", "Buy milk", json!({ "priority": 4, "labels": ["errands"],
                    "section_id": "s2",
                    "due": { "date": "2026-03-01", "string": "every day", "is_recurring": true } })),
                task("t2", "pweb", "Write copy", json!({ "priority": 4, "section_id": "s1",
                    "description": "First draft", "labels": ["Errands", "work"],
                    "due": { "date": "2026-03-02T09:30:00", "string": "Mar 2 9:30", "is_recurring": false },
                    "deadline": { "date": "2026-03-10" }, "duration": { "amount": 45, "unit": "minute" } })),
                task("t3", "pweb", "Proofread", json!({ "parent_id": "t2", "child_order": 1 })),
            ], "next_cursor": "page2" })).into_response()
        }
        "/reminders" => Json(json!({ "results": [
            { "item_id": "t2", "type": "relative", "minute_offset": 30 },
            { "item_id": "t2", "type": "absolute", "due": { "date": "2026-03-02T08:00:00" } },
            { "item_id": "t2", "type": "location" },
        ] }))
        .into_response(),
        "/comments" => {
            let attach = |name: &str, kind: &str, path: &str| {
                json!({
                "file_name": name, "file_type": kind, "file_url": format!("http://{host}{path}") })
            };
            Json(json!({ "results": match (q.get("task_id"), q.get("project_id")) {
                (Some(t), _) if t == "t2" => json!([
                    { "content": "Looks good", "posted_at": "2026-03-01T10:00:00Z" },
                    { "content": "", "posted_at": "2026-03-02T10:00:00Z", "file_attachment": attach("shot.png", "image/png", "/files/shot.png") },
                    { "content": "see doc", "posted_at": "2026-03-03T10:00:00Z", "file_attachment": attach("gone.pdf", "application/pdf", "/files/gone.pdf") },
                ]),
                (_, Some(p)) if p == "pweb" => json!([{ "content": "Kickoff notes", "posted_at": "2026-02-01T09:00:00Z" }]),
                (_, Some(p)) if p == "pi" => json!([{ "content": "inbox note", "posted_at": "2026-02-01T09:00:00Z" }]),
                _ => json!([]),
            } })).into_response()
        }
        "/files/shot.png" => PNG.into_response(),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn serve_fake() -> (String, Arc<Seen>) {
    let seen = Arc::new(Seen::default());
    let app = Router::new().fallback(fake).with_state(seen.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (url, seen)
}

/// A server reading `todoist`, with its user set up. Returns the router and a Device token.
async fn orbit(dir: &std::path::Path, todoist: &str) -> (Router, String) {
    let (app, code) = orbit_server::app_with(dir, todoist).unwrap();
    let body = json!({ "code": code.unwrap(), "password": "a long password", "name": "Test", "platform": "cli" });
    let res = call(&app, None, "POST", "/auth/setup", Some(body)).await.1;
    (app, res["token"].as_str().unwrap().into())
}

async fn call(
    app: &Router,
    token: Option<&str>,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(path);
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let mut req = req
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.map(|b| b.to_string()).unwrap_or_default()))
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1))));
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Starts a job and waits for its preview.
async fn preview(app: &Router, token: &str) -> (String, Value) {
    let (status, res) = call(
        app,
        Some(token),
        "POST",
        "/import/todoist",
        Some(json!({ "token": TOKEN })),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let job = res["job"].as_str().unwrap().to_string();
    for _ in 0..200 {
        let (_, p) = call(
            app,
            Some(token),
            "GET",
            &format!("/import/todoist/{job}"),
            None,
        )
        .await;
        if p["status"] != "running" {
            return (job, p);
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    panic!("import job never finished");
}

async fn rows(app: &Router, token: &str) -> Vec<Value> {
    let req = json!({ "v": 1, "commands": [] });
    call(app, Some(token), "POST", "/sync", Some(req)).await.1["delta"]
        .as_array()
        .unwrap()
        .clone()
}

fn of<'a>(rows: &'a [Value], kind: &str, field: &str, value: &str) -> &'a Value {
    rows.iter()
        .find(|r| r["kind"] == kind && r[field] == value)
        .unwrap_or_else(|| panic!("no {kind} with {field}={value}"))
}

#[tokio::test]
async fn previews_the_mapping_then_commits_and_reruns_create_only() {
    let (todoist, seen) = serve_fake().await;
    let dir = tempfile::tempdir().unwrap();
    let (app, token) = orbit(dir.path(), &todoist).await;

    let (job, p) = preview(&app, &token).await;
    assert_eq!(p["status"], "ready");
    let plan: Vec<[String; 4]> = p["projects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| ["id", "kind", "area", "name"].map(|f| x[f].as_str().unwrap().to_string()))
        .collect();
    let want = |a: &str, b: &str, c: &str, d: &str| [a, b, c, d].map(String::from);
    assert_eq!(
        plan,
        vec![
            want("pi", "inbox", "", ""),
            want("pw", "area", "Work", ""),
            want("pweb", "project", "Work", "Website"),
            want("plaunch", "project", "Work", "Website / Launch"),
        ]
    );
    assert_eq!(p["projects"][2]["tasks"], 3);
    assert_eq!(p["totals"]["tasks"], 6);
    assert_eq!(p["totals"]["attachments"], 2);

    // Edit the mapping: Launch becomes its own Area.
    let edited =
        json!({ "projects": [{ "id": "plaunch", "kind": "area", "area": "Launch", "name": "" }] });
    let (status, report) = call(
        &app,
        Some(&token),
        "POST",
        &format!("/import/todoist/{job}/commit"),
        Some(edited),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["tasks"], 6);
    assert_eq!(report["areas"], 2);
    assert_eq!(report["projects"], 1);
    assert_eq!(report["sections"], 1);
    assert_eq!(report["pages"], 1);
    assert_eq!(report["reminders"], 2);
    let issues = report["issues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i.as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    for note in [
        "Sorted",
        "comments of project",
        "every 2nd tuesday",
        "gone.pdf",
        "location reminder",
    ] {
        assert!(issues.contains(note), "{note} not in {issues}");
    }

    let all = rows(&app, &token).await;
    let area = |name: &str| of(&all, "area", "name", name)["id"].clone();
    let project = of(&all, "project", "name", "Website");
    assert_eq!(project["parent_id"], area("Work"));
    let section = of(&all, "section", "name", "Backlog");
    assert_eq!(section["parent_id"], project["id"]);

    let copy = of(&all, "task", "title", "Write copy");
    assert_eq!(copy["parent_id"], section["id"]);
    assert_eq!(copy["priority"], 1);
    assert_eq!(
        copy["due"],
        json!({ "date": "2026-03-02", "time": "09:30" })
    );
    assert_eq!(copy["deadline"], "2026-03-10");
    assert_eq!(copy["duration_min"], 45);
    assert_eq!(copy["label_ids"].as_array().unwrap().len(), 2);
    let desc = copy["description"].as_str().unwrap();
    assert!(
        desc.starts_with("First draft\n\n## Todoist comments"),
        "{desc}"
    );
    assert!(desc.contains("**2026-03-01**: Looks good"));
    assert!(
        desc.contains(&format!("[gone.pdf]({todoist}/files/gone.pdf)")),
        "URL kept: {desc}"
    );
    let blob = copy["blobs"][0].as_str().unwrap();
    assert!(desc.contains(&format!("![shot.png](blob:{blob})")));
    assert_eq!(copy["reminders"].as_array().unwrap().len(), 2);
    assert_eq!(copy["reminders"][0]["kind"], "before");
    assert_eq!(copy["reminders"][1]["at"], "2026-03-02T08:00");

    // Subtasks keep their order under the parent.
    let (a, b) = (
        of(&all, "task", "title", "Proofread"),
        of(&all, "task", "title", "Publish"),
    );
    assert_eq!(a["parent_id"], copy["id"]);
    assert_eq!(b["parent_id"], copy["id"]);
    assert!(a["order"].as_str().unwrap() < b["order"].as_str().unwrap());

    // Inbox task: P4 for API 1 stays default; recurrence parsed by the core.
    let milk = of(&all, "task", "title", "Buy milk");
    assert_eq!(milk["parent_id"], "00000000-0000-7000-8000-000000000001");
    assert_eq!(milk["recurrence"]["rule"], "day");
    assert_eq!(milk["recurrence"]["from_completion"], false);
    let report_task = of(&all, "task", "title", "Report");
    assert_eq!(report_task["priority"], 2);
    assert_eq!(report_task["recurrence"]["rule"], "2 weeks");
    assert_eq!(report_task["recurrence"]["from_completion"], true);

    // An unparseable recurrence is a dated Task with the original string and the Label.
    let standup = of(&all, "task", "title", "Standup");
    assert_eq!(standup["parent_id"], area("Launch"));
    assert_eq!(standup["recurrence"], Value::Null);
    assert_eq!(standup["due"]["date"], "2026-03-04");
    assert!(
        standup["description"]
            .as_str()
            .unwrap()
            .contains("Original recurrence: every 2nd tuesday of the month")
    );
    let label = of(&all, "label", "name", "imported-recurrence");
    assert_eq!(standup["label_ids"], json!([label["id"]]));

    // Labels merge by name, ignoring case; the Todoist-only label is new.
    assert_eq!(all.iter().filter(|r| r["kind"] == "label").count(), 3);
    // Todoist's named colours arrive as hex.
    assert_eq!(of(&all, "label", "name", "Errands")["color"], "#b8255f");
    assert_eq!(of(&all, "area", "name", "Work")["color"], "#4180ff");
    assert_eq!(of(&all, "project", "name", "Website")["color"], "#148fad");
    let page = of(&all, "page", "title", "Todoist comments");
    assert_eq!(page["parent_id"], project["id"]);
    assert!(page["body"].as_str().unwrap().contains("Kickoff notes"));

    // The blob is in the store, and the token was never written anywhere.
    let (status, _) = call(&app, Some(&token), "GET", &format!("/blobs/{blob}"), None).await;
    assert_eq!(status, StatusCode::OK);
    let mut stack = vec![dir.path().to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(d).unwrap().flatten() {
            let path = e.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let bytes = std::fs::read(&path).unwrap();
                assert!(
                    !bytes.windows(TOKEN.len()).any(|w| w == TOKEN.as_bytes()),
                    "token in {path:?}"
                );
            }
        }
    }
    {
        let seen = seen.0.lock().unwrap();
        assert!(
            seen.iter()
                .all(|(_, a)| a.as_deref() == Some(&format!("Bearer {TOKEN}")))
        );
        // The 429 was retried: /tasks was asked for twice before page 2.
        assert!(seen.iter().filter(|(p, _)| p.starts_with("/tasks")).count() >= 3);
    }

    // Re-run: only new Todoist items are created, and edits made in Orbit stay.
    let sync_edit = json!({ "v": 1, "commands": [{
        "uuid": "01900000-0000-7000-8000-0000000000e1", "type": "task.update",
        "args": { "id": milk["id"], "title": "x" }, "hlc": { "wall": 1, "counter": 0, "device": "01900000-0000-7000-8000-0000000000d1" } }] });
    call(&app, Some(&token), "POST", "/sync", Some(sync_edit)).await;
    let (job, p) = preview(&app, &token).await;
    assert_eq!(p["projects"][2]["imported"], 3);
    let (_, again) = call(
        &app,
        Some(&token),
        "POST",
        &format!("/import/todoist/{job}/commit"),
        Some(json!({})),
    )
    .await;
    assert_eq!(again["tasks"], 0);
    assert_eq!(again["existing_tasks"], 6);
    assert_eq!(again["areas"], 0);
    assert_eq!(again["pages"], 0);
    assert_eq!(again["labels"], 0);
    assert_eq!(
        rows(&app, &token)
            .await
            .iter()
            .filter(|r| r["kind"] == "task")
            .count(),
        6
    );
}

#[tokio::test]
async fn a_rejected_token_fails_the_job_without_echoing_it() {
    let dir = tempfile::tempdir().unwrap();
    let app_dir = dir.path();
    let rejecting = Router::new().fallback(|| async { StatusCode::UNAUTHORIZED });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, rejecting).await.unwrap() });
    let (app, token) = orbit(app_dir, &url).await;
    let (_, p) = preview(&app, &token).await;
    assert_eq!(p["status"], "failed");
    assert!(!p.to_string().contains(TOKEN));
    // Unknown jobs and a missing token are plain errors.
    let (status, _) = call(&app, Some(&token), "GET", "/import/todoist/nope", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(
        &app,
        Some(&token),
        "POST",
        "/import/todoist",
        Some(json!({ "token": " " })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = call(
        &app,
        None,
        "POST",
        "/import/todoist",
        Some(json!({ "token": "t" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
