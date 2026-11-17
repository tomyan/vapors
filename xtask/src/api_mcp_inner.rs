//! `xtask api-mcp-inner`: the session's MCP server, inside the sandbox
//! (`docs/plans/api-builder.md`). It speaks MCP's JSON-RPC over stdio to
//! the CLI and offers the harness's tools; each call is posted through
//! the relay to the harness server outside, which answers it. It is the
//! trusted binary, bound read-only.

use std::io::{BufRead, Read, Write};
use std::net::TcpStream;
use std::process::ExitCode;
use std::time::Duration;

use serde_json::{Value, json};

/// A tool the session may call: its name, what it does, its one string
/// argument (if any), and the harness route that answers it.
pub struct Tool {
    pub name: &'static str,
    pub description: &'static str,
    pub argument: Option<&'static str>,
    pub route: &'static str,
}

/// The harness's tools.
pub const TOOLS: [Tool; 1] = [Tool {
    name: "ask_lead",
    description: "Ask the lead a question you cannot settle from the brief or the repository. \
                  It waits up to 30 minutes for the answer; with none, write what blocks you in \
                  BLOCKED.md and stop.",
    argument: Some("question"),
    route: "/api-build/ask",
}];

/// How long a tool call waits for the harness: `ask_lead` waits 30
/// minutes for the lead (`api_ask.rs`), and a minute more lets that wait
/// end first and say so.
pub const CALL_DEADLINE: Duration = Duration::from_mins(31);

/// Serves MCP on stdio, posting tool calls to loopback `port`.
pub fn run(port: u16) -> ExitCode {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            return ExitCode::FAILURE;
        };
        let Some(reply) = answer(&line, port) else {
            continue;
        };
        if writeln!(stdout, "{reply}")
            .and_then(|()| stdout.flush())
            .is_err()
        {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

/// The reply to one JSON-RPC message; `None` for a notification.
pub fn answer(line: &str, port: u16) -> Option<Value> {
    let message: Value = match serde_json::from_str(line) {
        Ok(message) => message,
        Err(e) => return Some(error(&Value::Null, -32700, &format!("not JSON: {e}"))),
    };
    let id = message.get("id")?.clone();
    let result = match message["method"].as_str() {
        Some("initialize") => Ok(initialized(&message)),
        Some("tools/list") => Ok(json!({"tools": TOOLS.iter().map(listed).collect::<Vec<_>>()})),
        Some("tools/call") => Ok(called(&message["params"], port)),
        Some("ping") => Ok(json!({})),
        other => Err(format!("no method {other:?} here")),
    };
    Some(match result {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err(why) => error(&id, -32601, &why),
    })
}

fn initialized(message: &Value) -> Value {
    let version = message["params"]["protocolVersion"].clone();
    json!({
        "protocolVersion": version,
        "capabilities": {"tools": {}},
        "serverInfo": {"name": "api-build harness", "version": env!("CARGO_PKG_VERSION")},
    })
}

fn listed(tool: &Tool) -> Value {
    let (properties, required) = match tool.argument {
        Some(name) => (json!({name: {"type": "string"}}), json!([name])),
        None => (json!({}), json!([])),
    };
    json!({
        "name": tool.name,
        "description": tool.description,
        "inputSchema": {"type": "object", "properties": properties, "required": required},
    })
}

/// A tool call answered by the harness; a failure is the call's error
/// result, which the session reads.
fn called(params: &Value, port: u16) -> Value {
    let name = params["name"].as_str().unwrap_or_default();
    let said = match TOOLS.iter().find(|tool| tool.name == name) {
        Some(tool) => posted(port, tool.route, &params["arguments"]),
        None => Err(format!("no tool {name:?}")),
    };
    let (text, is_error) = match said {
        Ok(text) => (text, false),
        Err(why) => (why, true),
    };
    json!({"content": [{"type": "text", "text": text}], "isError": is_error})
}

/// `arguments` posted to the harness's `route`; the answer's body.
fn posted(port: u16, route: &str, arguments: &Value) -> Result<String, String> {
    let body = arguments.to_string();
    let mut stream =
        TcpStream::connect(("127.0.0.1", port)).map_err(|e| format!("the relay: {e}"))?;
    stream
        .set_read_timeout(Some(CALL_DEADLINE))
        .map_err(|e| e.to_string())?;
    let request = format!(
        "POST {route} HTTP/1.1\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("asking the harness: {e}"))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| format!("no answer from the harness within {CALL_DEADLINE:?}: {e}"))?;
    let (head, answer) = response
        .split_once("\r\n\r\n")
        .ok_or("an answer with no head")?;
    if head.starts_with("HTTP/1.1 200") {
        return Ok(answer.to_owned());
    }
    Err(answer.to_owned())
}

fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

#[cfg(test)]
#[path = "api_mcp_inner_tests.rs"]
mod tests;
