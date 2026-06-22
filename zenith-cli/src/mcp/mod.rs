//! `zenith mcp` — a minimal, dependency-light MCP server over stdio.
//!
//! Speaks JSON-RPC 2.0 line-delimited on stdin/stdout (the MCP stdio transport)
//! and exposes the `zenith` command surface as MCP tools. It is hand-rolled on
//! `serde_json` (already a dependency) rather than pulling an async MCP SDK, so
//! the binary stays small.
//!
//! Intended for remote / CI / production contexts where an agent cannot run a
//! local binary. For a local agent, the install-the-CLI-and-run-it path (the
//! `zenith` skill) is preferred — it is faster and cheaper on tokens. The
//! `initialize` response says as much in its `instructions`.
//!
//! Logs go to stderr; stdout is reserved for the JSON-RPC framing.

mod exec;
mod tools;

use std::io::{self, BufRead, Write};

use serde_json::{Value, json};

/// The MCP protocol revision this server defaults to when the client does not
/// request one.
const DEFAULT_PROTOCOL: &str = "2025-06-18";

/// Steering shown to clients on connect.
const INSTRUCTIONS: &str = "Zenith authors, validates, and renders deterministic .zen design \
documents. Prefer the local `zenith` CLI when available (install it and run commands directly) \
— it is faster and cheaper on tokens than these MCP tools. Use this server for remote, CI, or \
server contexts where a local binary is not available. Always `zenith_validate` before \
`zenith_render`, and prefer `zenith_tx` for edits to existing documents.";

/// Run the stdio MCP server until stdin closes. Always returns success.
pub fn run() -> u8 {
    let stdin = io::stdin();
    let mut out = io::stdout();
    let reader = stdin.lock();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("zenith mcp: stdin read error: {e}");
                break;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = handle_message(&line) {
            if writeln!(out, "{response}").is_err() {
                break;
            }
            let _ = out.flush();
        }
    }
    0
}

/// Handle one JSON-RPC message. Returns `Some(response)` for requests and
/// `None` for notifications (and for messages that need no reply).
///
/// Exposed for integration tests so the protocol can be driven without stdio.
pub fn handle_message(line: &str) -> Option<Value> {
    let msg: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return Some(error(Value::Null, -32700, &format!("parse error: {e}"))),
    };

    let id = msg.get("id").cloned();
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let params = msg.get("params").cloned().unwrap_or(Value::Null);

    // Notifications (no id) get no response.
    let id = id?;

    match method {
        "initialize" => Some(success(id, initialize_result(&params))),
        "ping" => Some(success(id, json!({}))),
        "tools/list" => Some(success(id, tools::list_payload())),
        "tools/call" => Some(tools_call(id, &params)),
        other => Some(error(id, -32601, &format!("method not found: {other}"))),
    }
}

/// Build the `initialize` result, echoing the client's protocol version when valid.
fn initialize_result(params: &Value) -> Value {
    let protocol = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_PROTOCOL);
    json!({
        "protocolVersion": protocol,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "zenith", "version": env!("CARGO_PKG_VERSION") },
        "instructions": INSTRUCTIONS,
    })
}

/// Execute a `tools/call`. Tool-execution failures are reported inside the
/// result (`isError: true`), per the MCP spec — only malformed requests are
/// JSON-RPC errors.
fn tools_call(id: Value, params: &Value) -> Value {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return error(id, -32602, "missing tool name");
    };
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    let result = exec::call(name, &args);
    success(
        id,
        json!({
            "content": [ { "type": "text", "text": result.text } ],
            "isError": result.is_error,
        }),
    )
}

fn success(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}
