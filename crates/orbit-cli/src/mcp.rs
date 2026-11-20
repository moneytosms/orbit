//! `orbit mcp`: a Model Context Protocol server over stdio. JSON-RPC 2.0, one message per line.
//! Each tool runs the same command as its CLI verb; a result's text is the `--json` body.
use crate::cmd;
use crate::out::{Done, Fail, usage};
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::process::ExitCode;

const PROTOCOL: &str = "2025-06-18";

/// Serves requests from stdin until it closes. Notifications get no reply.
pub fn serve() -> ExitCode {
    let mut stdout = std::io::stdout();
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = handle(&line) {
            let sent = writeln!(stdout, "{reply}").and_then(|_| stdout.flush());
            if sent.is_err() {
                break;
            }
        }
    }
    ExitCode::SUCCESS
}

/// The reply to one message, or None for a notification.
fn handle(line: &str) -> Option<Value> {
    let msg: Value = match serde_json::from_str(line) {
        Ok(msg) => msg,
        Err(e) => return Some(error(Value::Null, -32700, &format!("parse error: {e}"))),
    };
    let id = msg.get("id")?.clone();
    let method = msg["method"].as_str().unwrap_or_default();
    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "orbit", "version": env!("CARGO_PKG_VERSION") },
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "tools/call" => Ok(call(&msg["params"])),
        _ => Err((-32601, format!("method not found: {method}"))),
    };
    Some(match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, message)) => error(id, code, &message),
    })
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn schema(properties: Value, required: &[&str]) -> Value {
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

fn tools() -> Value {
    let text = json!({ "type": "string" });
    let dry_run =
        json!({ "type": "boolean", "description": "Report what would change; write nothing." });
    let all = json!({ "type": "boolean", "description": "Show every row, not the first 20." });
    let r = json!({ "type": "string", "description": "A row number from the last listing, or an id prefix of 6+ characters." });
    json!([
        { "name": "add", "description": "Add a task.",
          "inputSchema": schema(json!({
              "title": text, "id": { "type": "string", "description": "UUID for the task." }, "dry_run": dry_run,
          }), &["title"]) },
        { "name": "ls", "description": "List open tasks.",
          "inputSchema": schema(json!({ "all": all }), &[]) },
        { "name": "next", "description": "Show what to do next.",
          "inputSchema": schema(json!({ "all": all }), &[]) },
        { "name": "done", "description": "Complete a task.",
          "inputSchema": schema(json!({ "r": r, "dry_run": dry_run }), &["r"]) },
        { "name": "show", "description": "Show one task.",
          "inputSchema": schema(json!({ "r": r }), &["r"]) },
        { "name": "edit", "description": "Change a task's title or description.",
          "inputSchema": schema(json!({
              "r": r, "title": text, "description": text, "dry_run": dry_run,
          }), &["r"]) },
        { "name": "search", "description": "Find tasks by words.",
          "inputSchema": schema(json!({ "query": text, "all": all }), &["query"]) },
        { "name": "checkin", "description": "Check a routine in for a day (default today).",
          "inputSchema": schema(json!({
              "r": r, "date": { "type": "string", "description": "YYYY-MM-DD." }, "dry_run": dry_run,
          }), &["r"]) },
        { "name": "sync", "description": "Send queued changes to the server and fetch the rest.",
          "inputSchema": schema(json!({}), &[]) },
        { "name": "status", "description": "Show the server, sign-in state and unsynced count.",
          "inputSchema": schema(json!({}), &[]) },
    ])
}

/// Runs a tool call. Failures come back as results with `isError`, as the MCP spec asks.
fn call(params: &Value) -> Value {
    let name = params["name"].as_str().unwrap_or_default();
    match run_tool(name, &params["arguments"]) {
        Ok((_, mut body)) => {
            body["v"] = json!(1);
            tool_result(&body, false)
        }
        Err(f) => {
            let err = json!({ "code": f.code, "message": f.message, "hint": f.hint });
            tool_result(&json!({ "v": 1, "error": err }), true)
        }
    }
}

fn tool_result(body: &Value, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": body.to_string() }],
        "isError": is_error,
    })
}

fn text<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args[key].as_str()
}

fn required<'a>(args: &'a Value, key: &str) -> Result<&'a str, Fail> {
    text(args, key).ok_or_else(|| usage(format!("`{key}` is required")))
}

fn flag(args: &Value, key: &str) -> bool {
    args[key].as_bool().unwrap_or(false)
}

fn run_tool(name: &str, a: &Value) -> Done {
    match name {
        "add" => cmd::add(
            &[required(a, "title")?.to_string()],
            text(a, "id"),
            flag(a, "dry_run"),
        ),
        "ls" => cmd::ls(flag(a, "all")),
        "next" => cmd::next(flag(a, "all")),
        "done" => cmd::done(required(a, "r")?, flag(a, "dry_run")),
        "show" => cmd::show(required(a, "r")?),
        "edit" => cmd::edit(
            required(a, "r")?,
            text(a, "title"),
            text(a, "description"),
            flag(a, "dry_run"),
        ),
        "search" => {
            let words: Vec<String> = required(a, "query")?
                .split_whitespace()
                .map(String::from)
                .collect();
            cmd::search(&words, flag(a, "all"))
        }
        "checkin" => cmd::checkin(required(a, "r")?, text(a, "date"), flag(a, "dry_run")),
        "sync" => cmd::sync(),
        "status" => cmd::status(),
        _ => Err(usage(format!("unknown tool `{name}`"))),
    }
}
