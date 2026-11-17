//! One API request forwarded with the real key (`docs/plans/api-builder.md`):
//! curl sends it, reading its headers, the key among them, from its
//! stdin, so neither is in a command line; its body from a file in the
//! builder's private directory, outside the sandbox. The response's head
//! is passed back with the hop-by-hop headers dropped and `connection:
//! close`, its body streamed through as curl writes it, its usage read on
//! the way (`api_usage.rs`).

use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use crate::api_http::Request;
use crate::api_prices::Usage;
use crate::api_usage::Scan;
use crate::ux_secret::Secret;

/// Where requests go.
pub const API: &str = "https://api.anthropic.com";

/// The whole request, connection to last byte. The Anthropic SDK under
/// the CLI scales its own timeout up to 60 minutes for a large
/// `max_tokens`; the proxy's is that ceiling, so the client's fires first
/// and the proxy's only ends a request the client has stopped waiting for.
pub const REQUEST_DEADLINE: Duration = Duration::from_hours(1);

/// Request headers not forwarded: the key the session holds is a dummy,
/// and the rest belong to the hop, not the request.
const DROPPED: [&str; 8] = [
    "x-api-key",
    "authorization",
    "host",
    "content-length",
    "connection",
    "accept-encoding",
    "transfer-encoding",
    "expect",
];

/// How a forwarded request ended.
pub struct Forwarded {
    pub status: u16,
    /// The usage, for a response that holds one.
    pub usage: Option<Usage>,
}

/// `request` sent to the API with `secret`, its body staged at `staged`,
/// the response written to `client`.
///
/// # Errors
///
/// curl not starting or ending before a response head; each message has
/// passed the key guard.
pub fn forward(
    request: &Request,
    secret: &Secret,
    staged: &Path,
    client: &mut impl Write,
) -> Result<Forwarded, String> {
    std::fs::write(staged, &request.body)
        .map_err(|e| format!("staging the body at {}: {e}", staged.display()))?;
    let mut curl = started(request, secret, staged)?;
    let stdout = curl.stdout.take().ok_or("curl's stdout was not opened")?;
    let mut response = BufReader::new(stdout);
    let status =
        passed_head(&mut response, client).map_err(|why| ended(&mut curl, secret, &why))?;
    let mut scan = Scan::default();
    streamed(&mut response, client, &mut scan);
    let _ = curl.wait(); // the head arrived; the body's end is what was streamed
    let _ = std::fs::remove_file(staged);
    Ok(Forwarded {
        status,
        usage: scan.usage(),
    })
}

fn started(request: &Request, secret: &Secret, staged: &Path) -> Result<Child, String> {
    let mut command = Command::new("curl");
    command.args(arguments(&request.method, &request.target, staged));
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not run curl: {e}; fix: install curl"))?;
    let mut headers = secret.header_named("x-api-key");
    for (name, value) in request
        .headers
        .iter()
        .filter(|(name, _)| !DROPPED.contains(&name.as_str()))
    {
        let _ = writeln!(headers, "{name}: {value}"); // writing to a String cannot fail
    }
    headers.push_str("Expect:\n");
    let mut stdin = child.stdin.take().ok_or("curl's stdin was not opened")?;
    stdin
        .write_all(headers.as_bytes())
        .map_err(|e| format!("writing curl's headers: {e}"))?;
    Ok(child)
}

/// curl's arguments: never a header, never the key.
pub fn arguments(method: &str, target: &str, staged: &Path) -> Vec<String> {
    let deadline = REQUEST_DEADLINE.as_secs().to_string();
    let mut args: Vec<String> = ["-q", "--silent", "--show-error", "--no-buffer", "--http1.1"]
        .map(str::to_owned)
        .to_vec();
    args.extend(["--include", "--connect-timeout", "20", "--max-time"].map(str::to_owned));
    args.extend([
        deadline,
        "--request".into(),
        method.to_owned(),
        "--header".into(),
        "@-".into(),
    ]);
    args.extend(["--data-binary".into(), format!("@{}", staged.display())]);
    args.push(format!("{API}{target}"));
    args
}

/// The response's head copied to `client`, hop-by-hop headers dropped;
/// its status.
fn passed_head(response: &mut impl BufRead, client: &mut impl Write) -> Result<u16, String> {
    let mut line = String::new();
    response
        .read_line(&mut line)
        .map_err(|e| format!("reading the response: {e}"))?;
    let status = line
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| {
            format!(
                "a response that does not start with a status line: {:?}",
                line.trim()
            )
        })?;
    let mut head = line.clone();
    loop {
        line.clear();
        response
            .read_line(&mut line)
            .map_err(|e| format!("reading the response head: {e}"))?;
        if line.trim().is_empty() {
            break;
        }
        let name = line
            .split(':')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if !matches!(
            name.as_str(),
            "transfer-encoding" | "content-length" | "connection" | "keep-alive"
        ) {
            head.push_str(&line);
        }
    }
    head.push_str("connection: close\r\n\r\n");
    client
        .write_all(head.as_bytes())
        .map_err(|e| format!("answering the session: {e}"))?;
    Ok(status)
}

fn streamed(response: &mut impl Read, client: &mut impl Write, scan: &mut Scan) {
    let mut buffer = [0; 16 * 1024];
    loop {
        match response.read(&mut buffer) {
            Ok(0) | Err(_) => return,
            Ok(read) => {
                scan.read(&buffer[..read]);
                if client
                    .write_all(&buffer[..read])
                    .and_then(|()| client.flush())
                    .is_err()
                {
                    return; // the session closed the connection; curl ends with it
                }
            }
        }
    }
}

/// curl's failure, after `why`, its output guarded.
fn ended(curl: &mut Child, secret: &Secret, why: &str) -> String {
    let _ = curl.kill();
    let mut said = String::new();
    if let Some(mut stderr) = curl.stderr.take() {
        let _ = stderr.read_to_string(&mut said);
    }
    let said = secret
        .guard("curl's error", said.trim())
        .unwrap_or("(withheld: it held part of the key)");
    format!("{why}; curl said: {said}")
}

#[cfg(test)]
#[path = "api_upstream_tests.rs"]
mod tests;
