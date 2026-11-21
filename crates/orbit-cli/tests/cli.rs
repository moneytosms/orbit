//! The `orbit` binary end to end: `--json` snapshots, error envelopes, exit codes, and a real local server.
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;

const PW: &str = "a long password";
const A: &str = "aaaaaa00-0000-4000-8000-000000000001";
const B: &str = "aaaaaa00-0000-4000-8000-000000000002";

struct Out {
    code: i32,
    stdout: String,
    stderr: String,
}

impl Out {
    fn json(&self) -> Value {
        serde_json::from_str(&self.stdout)
            .unwrap_or_else(|e| panic!("bad json {e}: {}", self.stdout))
    }
}

/// Runs the binary with its own data dir and session, never a terminal, never a prompt.
fn orbit(dir: &Path, args: &[&str]) -> Out {
    let out = Command::new(env!("CARGO_BIN_EXE_orbit"))
        .args(args)
        .env("ORBIT_DIR", dir)
        .env("ORBIT_SESSION", "test")
        .env("ORBIT_PASSWORD", PW)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    Out {
        code: out.status.code().unwrap(),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}

/// A server with its user already set up, on its own thread. Returns its URL and data dir.
/// Its Todoist is an address nothing listens on.
fn server() -> (String, tempfile::TempDir) {
    server_at("http://127.0.0.1:1")
}

fn server_at(todoist: &str) -> (String, tempfile::TempDir) {
    let todoist = todoist.to_string();
    let data = tempfile::tempdir().unwrap();
    let path = data.path().to_path_buf();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let (app, code) = orbit_server::app_with(&path, &todoist).unwrap();
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap());
            let app = app.into_make_service_with_connect_info::<SocketAddr>();
            tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
            let body = json!({ "code": code.unwrap(), "password": PW, "name": "Setup", "platform": "cli" });
            reqwest::Client::new()
                .post(format!("{url}/auth/setup"))
                .json(&body)
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap();
            tx.send(url).unwrap();
            std::future::pending::<()>().await;
        });
    });
    (rx.recv().unwrap(), data)
}

#[test]
fn empty_list_says_no_tasks() {
    let d = tempfile::tempdir().unwrap();
    let o = orbit(d.path(), &["ls"]);
    assert_eq!((o.code, o.stdout.as_str()), (0, "no tasks\n"));
    let o = orbit(d.path(), &["--json", "ls"]);
    assert_eq!(o.code, 0);
    assert_eq!(
        o.stdout,
        "{\"items\":[],\"shown\":0,\"total\":0,\"truncated\":false,\"v\":1}\n"
    );
    assert_eq!(
        o.json(),
        json!({ "v": 1, "items": [], "total": 0, "shown": 0, "truncated": false })
    );
}

#[test]
fn no_args_print_help_and_show_names_missing_ref() {
    let d = tempfile::tempdir().unwrap();
    let no_args = orbit(d.path(), &[]);
    assert_eq!(no_args.code, 0);
    assert!(no_args.stdout.contains("Usage: orbit"));
    assert!(no_args.stderr.is_empty());

    let missing = orbit(d.path(), &["show"]);
    assert_eq!(missing.code, 2);
    assert!(missing.stderr.contains("<R>"), "{}", missing.stderr);
}

#[test]
fn add_then_ls_snapshot() {
    let d = tempfile::tempdir().unwrap();
    let o = orbit(d.path(), &["--json", "add", "buy", "milk"]);
    assert_eq!(o.code, 0);
    let added = o.json();
    assert_eq!(added["v"], 1);
    assert_eq!(added["title"], "buy milk");
    assert_eq!(added["created"], true);
    assert_eq!(added["dry_run"], false);

    let o = orbit(d.path(), &["--json", "ls"]);
    let body = o.json();
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["n"], 1);
    assert_eq!(body["items"][0]["title"], "buy milk");
    assert_eq!(body["items"][0]["id"], added["id"]);
    assert_eq!(body["truncated"], false);

    let human = orbit(d.path(), &["ls"]);
    assert!(human.stdout.starts_with("#  TITLE"), "{}", human.stdout);
    assert!(human.stdout.contains("buy milk"));
}

#[test]
fn add_applies_quick_add_syntax() {
    let d = tempfile::tempdir().unwrap();
    let added = orbit(d.path(), &["--json", "add", "pay rent tomorrow p1"]).json();
    assert_eq!(added["title"], "pay rent");
    let task = &orbit(d.path(), &["--json", "show", added["id"].as_str().unwrap()]).json()["task"];
    assert_eq!(task["priority"], 1, "{task}");
    assert!(task["due"]["date"].is_string(), "{task}");

    let only_tokens = orbit(d.path(), &["--json", "add", "p1"]);
    assert_eq!(only_tokens.code, 1);
    assert_eq!(only_tokens.json()["error"]["code"], "title_required");
}

#[test]
fn add_normalizes_markdown_links_and_keeps_url_in_description() {
    let d = tempfile::tempdir().unwrap();
    let added = orbit(
        d.path(),
        &[
            "--json",
            "add",
            "[Notion – all-in-one](https://notion.so/workspace?ref=abc)",
        ],
    )
    .json();
    assert_eq!(added["title"], "Notion – all-in-one");
    let task = &orbit(d.path(), &["--json", "show", added["id"].as_str().unwrap()]).json()["task"];
    assert_eq!(task["description"], "https://notion.so/workspace?ref=abc");
}

#[test]
fn add_normalizes_only_url_only_titles() {
    let d = tempfile::tempdir().unwrap();
    let added = orbit(d.path(), &["--json", "add", "https://example.test/task"]).json();
    assert_eq!(added["title"], "example.test/task");
    let task = &orbit(d.path(), &["--json", "show", added["id"].as_str().unwrap()]).json()["task"];
    assert_eq!(task["description"], "https://example.test/task");

    let mixed = orbit(
        d.path(),
        &[
            "--json",
            "add",
            "https://example.test/task and remember this",
        ],
    )
    .json();
    assert_eq!(
        mixed["title"],
        "https://example.test/task and remember this"
    );
    let task = &orbit(d.path(), &["--json", "show", mixed["id"].as_str().unwrap()]).json()["task"];
    assert_eq!(task["description"], "");
}

#[test]
fn add_with_id_is_idempotent() {
    let d = tempfile::tempdir().unwrap();
    let first = orbit(d.path(), &["--json", "add", "--id", A, "once"]).json();
    assert_eq!(first["created"], true);
    let again = orbit(d.path(), &["--json", "add", "--id", A, "once"]);
    assert_eq!(again.code, 0);
    assert_eq!(again.json()["created"], false);
    let body = orbit(d.path(), &["--json", "ls"]).json();
    assert_eq!(body["total"], 1);
}

#[test]
fn dry_run_changes_nothing() {
    let d = tempfile::tempdir().unwrap();
    let o = orbit(d.path(), &["--json", "add", "--dry-run", "maybe"]);
    assert_eq!(o.code, 0);
    assert_eq!(o.json()["dry_run"], true);
    assert_eq!(orbit(d.path(), &["--json", "ls"]).json()["total"], 0);
}

#[test]
fn truncation_hints_and_all() {
    let d = tempfile::tempdir().unwrap();
    for i in 0..21 {
        orbit(d.path(), &["add", &format!("task {i:02}")]);
    }
    let human = orbit(d.path(), &["ls"]);
    assert!(
        human
            .stdout
            .ends_with("showing 20/21, pass --all to see the rest\n"),
        "{}",
        human.stdout
    );
    let body = orbit(d.path(), &["--json", "ls"]).json();
    assert_eq!(body["shown"], 20);
    assert_eq!(body["truncated"], true);
    assert_eq!(body["hint"], "showing 20/21, pass --all to see the rest");
    let all = orbit(d.path(), &["--json", "ls", "--all"]).json();
    assert_eq!(all["items"].as_array().unwrap().len(), 21);
    assert_eq!(all["truncated"], false);
}

#[test]
fn refs_by_row_number_and_prefix() {
    let d = tempfile::tempdir().unwrap();
    orbit(d.path(), &["add", "--id", A, "first"]);
    orbit(
        d.path(),
        &[
            "add",
            "--id",
            "bbbbbb00-0000-4000-8000-000000000003",
            "second",
        ],
    );
    orbit(d.path(), &["ls"]);
    let by_row = orbit(d.path(), &["--json", "show", "2"]).json();
    assert_eq!(by_row["task"]["title"], "second");
    let by_prefix = orbit(d.path(), &["--json", "show", "aaaaaa"]).json();
    assert_eq!(by_prefix["task"]["title"], "first");
}

#[test]
fn wide_listing_prints_an_usable_short_id_and_raw_description() {
    let d = tempfile::tempdir().unwrap();
    orbit(
        d.path(),
        &["add", "--id", A, "[source title](https://example.test/raw)"],
    );
    let listing = orbit(d.path(), &["ls", "--wide"]);
    assert!(listing.stdout.contains("DESCRIPTION / SOURCE"));
    assert!(listing.stdout.contains("https://example.test/raw"));
    assert!(listing.stdout.contains("  aaaa  https://example.test/raw"));
    let shown = orbit(d.path(), &["--json", "show", "aaaa"]);
    assert_eq!(shown.json()["task"]["title"], "source title");
}

#[test]
fn numeric_only_wide_prefix_resolves_as_id_after_row_lookup() {
    let d = tempfile::tempdir().unwrap();
    let id = "12345678-0000-4000-8000-000000000001";
    orbit(d.path(), &["add", "--id", id, "numeric id"]);
    let listing = orbit(d.path(), &["ls", "--wide"]);
    let prefix = listing
        .stdout
        .lines()
        .nth(1)
        .unwrap()
        .split_whitespace()
        .last()
        .unwrap();
    assert_eq!(prefix, "1234");
    let shown = orbit(d.path(), &["--json", "show", prefix]);
    assert_eq!(shown.json()["task"]["id"], id);
}

#[test]
fn unmatched_hex_words_fall_through_to_title_match() {
    let d = tempfile::tempdir().unwrap();
    orbit(d.path(), &["add", "Cafe"]);
    orbit(d.path(), &["add", "beef"]);
    let cafe = orbit(d.path(), &["--json", "show", "Cafe"]);
    assert_eq!(cafe.json()["task"]["title"], "Cafe");
    let beef = orbit(d.path(), &["--json", "show", "beef"]);
    assert_eq!(beef.json()["task"]["title"], "beef");
}

#[test]
fn ambiguous_ref_is_an_error() {
    let d = tempfile::tempdir().unwrap();
    orbit(d.path(), &["add", "--id", A, "one"]);
    orbit(d.path(), &["add", "--id", B, "two"]);
    let o = orbit(d.path(), &["--json", "done", "aaaaaa"]);
    assert_eq!(o.code, 1);
    assert_eq!(o.json()["error"]["code"], "ambiguous");
    assert!(
        o.json()["error"]["hint"]
            .as_str()
            .unwrap()
            .contains("longer id prefix")
    );
}

#[test]
fn title_refs_and_ambiguous_candidates() {
    let d = tempfile::tempdir().unwrap();
    orbit(d.path(), &["add", "buy oat milk"]);
    orbit(d.path(), &["add", "buy almond milk"]);
    orbit(d.path(), &["ls"]);
    let unique = orbit(d.path(), &["--json", "show", "oat milk"]);
    assert_eq!(unique.json()["task"]["title"], "buy oat milk");
    let ambiguous = orbit(d.path(), &["--json", "show", "milk"]);
    assert_eq!(ambiguous.json()["error"]["code"], "ambiguous");
    let body = ambiguous.json();
    let message = body["error"]["message"].as_str().unwrap();
    assert!(message.contains("1: buy oat milk"), "{message}");
    assert!(message.contains("2: buy almond milk"), "{message}");
}

#[test]
fn stale_listing_explains_refresh() {
    let d = tempfile::tempdir().unwrap();
    orbit(d.path(), &["add", "one"]);
    orbit(d.path(), &["ls"]);
    let path = d.path().join("listing.json");
    let mut listing: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    listing["listed_at"] = json!(now - 3 * 24 * 60 * 60);
    std::fs::write(&path, serde_json::to_vec(&listing).unwrap()).unwrap();
    let stale = orbit(d.path(), &["--json", "show", "1"]);
    assert_eq!(stale.code, 1);
    assert_eq!(stale.json()["error"]["code"], "stale_listing");
    assert!(
        stale.json()["error"]["message"]
            .as_str()
            .unwrap()
            .contains("listing is 3 days old")
    );
    assert_eq!(stale.json()["error"]["hint"], "run `orbit ls`");
}

#[test]
fn parallel_terminals_leave_a_valid_shared_listing() {
    let d = tempfile::tempdir().unwrap();
    orbit(d.path(), &["add", "first"]);
    orbit(d.path(), &["add", "second"]);
    let dir = d.path().to_path_buf();
    let readers: Vec<_> = (0..8)
        .map(|_| {
            let dir = dir.clone();
            std::thread::spawn(move || orbit(&dir, &["ls"]))
        })
        .collect();
    for reader in readers {
        assert_eq!(reader.join().unwrap().code, 0);
    }
    let listing: Value =
        serde_json::from_slice(&std::fs::read(dir.join("listing.json")).unwrap()).unwrap();
    assert_eq!(listing["ids"].as_array().unwrap().len(), 2);
    assert!(listing["listed_at"].as_u64().is_some());
}

#[test]
fn done_and_edit_change_the_task() {
    let d = tempfile::tempdir().unwrap();
    orbit(d.path(), &["add", "--id", A, "draft"]);
    orbit(d.path(), &["ls"]);
    let o = orbit(d.path(), &["--json", "edit", "1", "--title", "final"]);
    assert_eq!(o.json()["changed"], true);
    let o = orbit(d.path(), &["--json", "done", "1"]);
    assert_eq!(o.json()["changed"], true);
    let again = orbit(d.path(), &["--json", "done", "1"]);
    assert_eq!(again.json()["changed"], false);
    assert_eq!(orbit(d.path(), &["--json", "ls"]).json()["total"], 0);
    let shown = orbit(d.path(), &["--json", "show", "aaaaaa"]).json();
    assert_eq!(shown["task"]["title"], "final");
}

#[test]
fn search_finds_tasks() {
    let d = tempfile::tempdir().unwrap();
    orbit(d.path(), &["add", "buy milk"]);
    orbit(d.path(), &["add", "walk dog"]);
    let body = orbit(d.path(), &["--json", "search", "milk"]).json();
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["title"], "buy milk");
}

#[test]
fn error_envelope_snapshot_and_exit_codes() {
    let d = tempfile::tempdir().unwrap();
    let o = orbit(d.path(), &["--json", "done", "zzz"]);
    assert_eq!(o.code, 1);
    assert_eq!(o.json()["v"], 1);
    assert_eq!(o.json()["error"]["code"], "not_found");
    assert_eq!(o.json()["error"]["hint"], "run `orbit ls`");

    let o = orbit(d.path(), &["--json", "done", "999"]);
    assert_eq!(o.code, 1);
    assert_eq!(o.json()["error"]["code"], "not_found");

    let o = orbit(d.path(), &["done", "999"]);
    assert_eq!(o.code, 1);
    assert_eq!(o.stdout, "");
    assert!(o.stderr.starts_with("error: no row 999"), "{}", o.stderr);
    assert!(o.stderr.contains("hint: "), "{}", o.stderr);
}

#[test]
fn bad_arguments_are_usage_errors() {
    let d = tempfile::tempdir().unwrap();
    let o = orbit(d.path(), &["--json", "frobnicate"]);
    assert_eq!(o.code, 2);
    assert_eq!(o.json()["v"], 1);
    assert_eq!(o.json()["error"]["code"], "usage");
}

#[test]
fn logout_needs_yes_when_changes_are_unsynced() {
    let (url, _server) = server();
    let d = tempfile::tempdir().unwrap();
    assert_eq!(orbit(d.path(), &["login", &url]).code, 0);
    orbit(d.path(), &["add", "pending"]);
    let o = orbit(d.path(), &["--json", "logout"]);
    assert_eq!(o.code, 1);
    assert_eq!(o.json()["error"]["code"], "confirm");
    let o = orbit(d.path(), &["--json", "logout", "--yes"]);
    assert_eq!(o.code, 0);
    assert_eq!(o.json()["signed_in"], false);
}

#[test]
fn login_sync_and_status_round_trip() {
    let (url, _server) = server();
    let d = tempfile::tempdir().unwrap();
    let o = orbit(d.path(), &["--json", "status"]);
    assert_eq!(o.json()["signed_in"], false);

    let o = orbit(d.path(), &["login", &url]);
    assert_eq!(o.code, 0, "{}", o.stderr);

    orbit(d.path(), &["add", "synced task"]);
    let o = orbit(d.path(), &["--json", "sync"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let body = o.json();
    assert_eq!(body["sent"], 1);
    assert_eq!(body["rejected"], json!([]));

    let st = orbit(d.path(), &["--json", "status"]).json();
    assert_eq!(st["signed_in"], true);
    assert_eq!(st["unsynced"], 0);
    assert_eq!(st["server"], url.as_str());
}

#[test]
fn wrong_password_is_a_credentials_error() {
    let (url, _server) = server();
    let d = tempfile::tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_orbit"))
        .args(["--json", "login", &url])
        .env("ORBIT_DIR", d.path())
        .env("ORBIT_PASSWORD", "the wrong password here")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["error"]["code"], "credentials");
}

#[test]
fn sync_without_login_explains_itself() {
    let d = tempfile::tempdir().unwrap();
    let o = orbit(d.path(), &["--json", "sync"]);
    assert_eq!(o.code, 1);
    assert_eq!(
        o.json()["error"],
        json!({ "code": "no_server", "message": "not logged in to a server", "hint": "run `orbit login <url>`" })
    );
}

/// A Todoist with one project, one task and nothing else.
fn fake_todoist() -> String {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let app = axum::Router::new().fallback(|uri: axum::http::Uri| async move {
                axum::Json(match uri.path() {
                    "/projects" => json!({ "results": [{ "id": "p1", "name": "Home" }] }),
                    "/tasks" => json!({ "results": [{ "id": "t1", "project_id": "p1", "content": "Fix tap" }] }),
                    _ => json!({ "results": [] }),
                })
            });
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            tx.send(format!("http://{}", listener.local_addr().unwrap())).unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });
    rx.recv().unwrap()
}

#[test]
fn import_todoist_previews_then_imports_with_yes() {
    let (url, _data) = server_at(&fake_todoist());
    let d = tempfile::tempdir().unwrap();
    assert_eq!(orbit(d.path(), &["login", &url]).code, 0);
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_orbit"))
            .args(args)
            .env("ORBIT_DIR", d.path())
            .env("ORBIT_SESSION", "test")
            .env("ORBIT_TODOIST_TOKEN", "fake-token")
            .stdin(Stdio::null())
            .output()
            .unwrap();
        Out {
            code: out.status.code().unwrap(),
            stdout: String::from_utf8(out.stdout).unwrap(),
            stderr: String::from_utf8(out.stderr).unwrap(),
        }
    };
    let o = run(&[
        "--json",
        "import",
        "todoist",
        "--map",
        "p1=project:Life/Tap",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let preview = o.json();
    assert_eq!(preview["committed"], false);
    assert_eq!(preview["preview"]["projects"][0]["kind"], "project");
    assert_eq!(preview["preview"]["projects"][0]["area"], "Life");
    assert_eq!(orbit(d.path(), &["--json", "sync"]).json()["sent"], 0);
    assert_eq!(orbit(d.path(), &["--json", "ls"]).json()["total"], 0);

    let o = run(&["--json", "import", "todoist", "--yes"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(o.json()["report"]["tasks"], 1);
    orbit(d.path(), &["sync"]);
    let ls = orbit(d.path(), &["--json", "ls"]).json();
    assert_eq!(ls["items"][0]["title"], "Fix tap");

    let o = run(&["import", "todoist", "--map", "nonsense"]);
    assert_eq!(o.code, 2);
}

#[test]
fn guide_has_its_sections() {
    let d = tempfile::tempdir().unwrap();
    let o = orbit(d.path(), &["guide"]);
    assert_eq!(o.code, 0);
    for section in [
        "Core commands",
        "Refs",
        "Filters and listings",
        "JSON (--json",
        "Exit codes",
    ] {
        assert!(o.stdout.contains(section), "missing {section}");
    }
}

#[test]
fn completions_print_for_each_shell() {
    let d = tempfile::tempdir().unwrap();
    for shell in ["bash", "zsh", "fish"] {
        let o = orbit(d.path(), &["completions", shell]);
        assert_eq!(o.code, 0, "{shell}: {}", o.stderr);
        assert!(o.stdout.contains("orbit"), "{shell} completions name orbit");
    }
}

#[test]
fn export_writes_a_zip_with_items() {
    let (url, _server) = server();
    let d = tempfile::tempdir().unwrap();
    let o = orbit(d.path(), &["login", &url]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    orbit(d.path(), &["add", "exported task"]);
    assert_eq!(orbit(d.path(), &["sync"]).code, 0);

    let file = d.path().join("out.zip");
    let o = orbit(d.path(), &["--json", "export", file.to_str().unwrap()]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let body = o.json();
    let bytes = std::fs::read(&file).unwrap();
    assert_eq!(body["v"], 1);
    assert_eq!(body["path"], file.to_str().unwrap());
    assert_eq!(body["bytes"], bytes.len());
    assert_eq!(&bytes[..2], b"PK");
    assert!(bytes.windows(10).any(|w| w == b"items.json"));
}

#[test]
fn export_without_login_explains_itself() {
    let d = tempfile::tempdir().unwrap();
    let o = orbit(d.path(), &["--json", "export", "x.zip"]);
    assert_eq!(o.code, 1);
    assert_eq!(o.json()["error"]["code"], "no_server");
}
