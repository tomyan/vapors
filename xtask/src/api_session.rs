//! The session itself (`docs/plans/api-builder.md`): `claude -p --bare`
//! inside the sandbox, the brief on its stdin, its JSON result written to
//! a file outside. It is stopped at its deadline, and as soon as the
//! proxy's budget is spent or its sum is no longer known.

use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::api_budget::Budget;

/// The whole session. F2.1, the largest slice an API builder has run,
/// took about 2.5 hours (`~/.cache/handoffs/api-builder.sh`); 4 hours
/// leaves room for a slower one and ends a session that is stuck.
pub const SESSION_DEADLINE: Duration = Duration::from_hours(4);

/// How often the session is checked against its deadline and budget.
const POLL: Duration = Duration::from_secs(1);

/// The tools the session may use without asking: everything a builder
/// needs, as the sandbox, not the tool list, is the fence; and the
/// harness's, from the MCP server below.
const TOOLS: [&str; 8] = [
    "Bash",
    "Read",
    "Edit",
    "Write",
    "Glob",
    "Grep",
    "mcp__harness__ask_lead",
    "mcp__harness__request_push",
];

/// The harness's MCP server, the trusted xtask inside, posting through
/// the relay; the only MCP server the session loads.
fn mcp_config() -> String {
    let server = serde_json::json!({
        "command": format!("{}/xtask", crate::api_sandbox::BIN),
        "args": ["api-mcp-inner", "--port", crate::api_sandbox::RELAY_PORT.to_string()],
    });
    serde_json::json!({"mcpServers": {"harness": server}}).to_string()
}

/// How the session ended.
#[derive(Debug, PartialEq, Eq)]
pub enum Ended {
    Exited(Option<i32>),
    /// Stopped by the harness, with why.
    Stopped(String),
}

/// The CLI's arguments after the program: the model, the CLI's own
/// budget (the proxy's holds too), JSON output.
pub fn arguments(model: &str, budget: f64) -> Vec<String> {
    let mut args: Vec<String> = ["-p", "--bare", "--model", model]
        .map(str::to_owned)
        .to_vec();
    args.extend(["--max-budget-usd".into(), budget.to_string()]);
    args.extend(
        [
            "--permission-mode",
            "dontAsk",
            "--output-format",
            "json",
            "--allowedTools",
        ]
        .map(str::to_owned),
    );
    args.extend(TOOLS.map(str::to_owned));
    args.extend([
        "--strict-mcp-config".into(),
        "--mcp-config".into(),
        mcp_config(),
    ]);
    args
}

/// `session` run with `brief` on its stdin and its stdout to `result`,
/// stopped at the deadline or when `budget` says no more.
///
/// # Errors
///
/// The brief or result file not opened, or the session not starting.
pub fn run(
    mut session: Command,
    (brief, result): (&Path, &Path),
    budget: &Budget,
) -> Result<Ended, String> {
    let stdin =
        File::open(brief).map_err(|e| format!("opening the brief {}: {e}", brief.display()))?;
    let stdout = File::create(result).map_err(|e| format!("creating {}: {e}", result.display()))?;
    let mut child = session
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("starting the session under bwrap: {e}"))?;
    if let Some(stderr) = child.stderr.take() {
        thread::spawn(move || escaped_lines(stderr));
    }
    let started = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| format!("waiting for the session: {e}"))?
        {
            return Ok(Ended::Exited(status.code()));
        }
        if let Some(why) = stop_reason(started, budget) {
            let _ = child.kill(); // bwrap's --die-with-parent ends the rest
            let _ = child.wait();
            return Ok(Ended::Stopped(why));
        }
        thread::sleep(POLL);
    }
}

/// The session's stderr, which any process inside can write, printed a
/// line at a time with every control character escaped, so none of it
/// reaches the terminal raw (review of #476-#478, M4).
fn escaped_lines(stderr: std::process::ChildStderr) {
    escaped_to(stderr, &mut std::io::stderr());
}

/// Each line of `from` written to `to` as `session: <line>`, escaped.
fn escaped_to(from: impl std::io::Read, to: &mut impl std::io::Write) {
    let mut from = std::io::BufReader::new(from);
    while let Some(line) = bounded_line(&mut from) {
        let line = String::from_utf8_lossy(&line);
        if writeln!(to, "session: {}", crate::api_refusals::escaped(&line)).is_err() {
            return; // the terminal is gone; nothing is left to protect
        }
    }
}

/// The most bytes of one stderr line held: the rest, to its newline, is
/// read and dropped, so a process that writes without a newline cannot
/// grow xtask's memory, which is outside the scope's limit (review of
/// #476-#478, N4). `api_refusals::escaped` prints 200 characters of it.
const LINE_BYTES: usize = 4096;

/// The next line of `from` without its newline, at most [`LINE_BYTES`];
/// a last line with no newline too; `None` at the end or on an error.
fn bounded_line(from: &mut impl std::io::BufRead) -> Option<Vec<u8>> {
    let mut line = Vec::new();
    loop {
        let buffer = from.fill_buf().ok()?;
        if buffer.is_empty() {
            return (!line.is_empty()).then_some(line);
        }
        let newline = buffer.iter().position(|&byte| byte == b'\n');
        let taken = newline.map_or(buffer.len(), |at| at + 1);
        let room = LINE_BYTES.saturating_sub(line.len());
        line.extend_from_slice(&buffer[..newline.unwrap_or(taken).min(room)]);
        from.consume(taken);
        if newline.is_some() {
            return Some(line);
        }
    }
}

fn stop_reason(started: Instant, budget: &Budget) -> Option<String> {
    if started.elapsed() >= SESSION_DEADLINE {
        return Some(format!(
            "the session reached its deadline of {SESSION_DEADLINE:?}"
        ));
    }
    budget.stopped()
}

#[cfg(test)]
#[path = "api_session_tests.rs"]
mod tests;
