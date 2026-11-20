//! `orbit mcp` end to end: the real binary speaks JSON-RPC 2.0 over stdio.
use serde_json::{Value, json};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Starts `orbit mcp` with its own data dir, sends each request as a line, returns the reply lines.
fn session(dir: &Path, requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_orbit"))
        .arg("mcp")
        .env("ORBIT_DIR", dir)
        .env("ORBIT_SESSION", "mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        for request in requests {
            writeln!(stdin, "{request}").unwrap();
        }
    }
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

/// The `--json` body a tool result carries.
fn tool_body(reply: &Value) -> Value {
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    let text = reply["result"]["content"][0]["text"].as_str().unwrap();
    serde_json::from_str(text).unwrap()
}

#[test]
fn lists_tools_and_runs_add_then_ls() {
    let d = tempfile::tempdir().unwrap();
    let replies = session(
        d.path(),
        &[
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize",
                    "params": { "protocolVersion": "2025-06-18", "capabilities": {},
                                "clientInfo": { "name": "test", "version": "0" } } }),
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
            json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call",
                    "params": { "name": "add", "arguments": { "title": "buy milk" } } }),
            json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/call",
                    "params": { "name": "ls", "arguments": {} } }),
        ],
    );
    assert_eq!(replies.len(), 4, "{replies:?}");
    assert_eq!(replies[0]["result"]["serverInfo"]["name"], "orbit");

    let tools: Vec<&str> = replies[1]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    for name in [
        "add", "ls", "next", "done", "show", "edit", "search", "checkin", "sync", "status",
    ] {
        assert!(tools.contains(&name), "{name} missing from {tools:?}");
    }
    for name in ["login", "logout", "import"] {
        assert!(!tools.contains(&name), "{name} must not be a tool");
    }

    let added = tool_body(&replies[2]);
    assert_eq!(added["v"], 1);
    assert_eq!(added["title"], "buy milk");
    assert_eq!(added["created"], true);

    let listed = tool_body(&replies[3]);
    assert_eq!(listed["total"], 1);
    assert_eq!(listed["items"][0]["title"], "buy milk");
}

#[test]
fn unknown_method_is_a_jsonrpc_error() {
    let d = tempfile::tempdir().unwrap();
    let replies = session(
        d.path(),
        &[json!({ "jsonrpc": "2.0", "id": 7, "method": "nope" })],
    );
    assert_eq!(replies[0]["id"], 7);
    assert_eq!(replies[0]["error"]["code"], -32601);
}
