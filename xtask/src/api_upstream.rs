//! One API request forwarded with the real key (`docs/plans/api-builder.md`):
//! curl sends it, reading its headers, the key among them, from its
//! stdin, so neither is in a command line; its body from a file in the
//! builder's private directory, outside the sandbox. The response's head
//! is passed back with the hop-by-hop headers dropped and `connection:
//! close`, its body streamed through as curl writes it, its usage read on
//! the way (`api_usage.rs`).

use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use crate::api_curls::Curls;
use crate::api_http::Request;
use crate::api_usage::{Counted, Scan};
use crate::ux_secret::Secret;

/// Where requests go.
pub const API: &str = "https://api.anthropic.com";

/// The whole request, connection to last byte. The Anthropic SDK under
/// the CLI scales its own timeout up to 60 minutes for a large
/// `max_tokens`; the proxy's is that ceiling, so the client's fires first
/// and the proxy's only ends a request the client has stopped waiting for.
pub const REQUEST_DEADLINE: Duration = Duration::from_hours(1);

/// The only variable curl's environment holds: where to find curl.
const CURL_PATH: &str = "/usr/bin:/bin";

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
    /// What the response counted, judged by curl's exit too (review of
    /// #476-#478, M3).
    pub counted: Counted,
}

/// A staged body, removed when it is dropped, on every path (review of
/// #476-#478, M7). At most one per request in flight, each at most
/// `api_http::BODY_LIMIT`: the proxy's in-flight cap bounds both their
/// count and their bytes.
pub struct Staged(PathBuf);

impl Staged {
    /// `body` written to `path`.
    ///
    /// # Errors
    ///
    /// The write failing, naming the path.
    pub fn new(path: PathBuf, body: &[u8]) -> Result<Self, String> {
        std::fs::write(&path, body)
            .map_err(|e| format!("staging the body at {}: {e}", path.display()))?;
        Ok(Self(path))
    }
}

impl Drop for Staged {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(&self.0) {
            eprintln!(
                "api-build: the staged body {} was not removed: {error}",
                self.0.display()
            );
        }
    }
}

/// `request` sent to the API with `secret`, its body at `staged`, the
/// response written to `client`. If the client stops reading (its write
/// deadline, set by the server) curl is killed, and the response counts as
/// cut, as it does when curl exits other than 0 or the stream does not end
/// whole.
///
/// # Errors
///
/// curl not starting or ending before a response head; each message has
/// passed the key guard and the escaping.
pub fn forward(
    request: &Request,
    (secret, curls): (&Secret, &Curls),
    staged: &Staged,
    client: &mut impl Write,
) -> Result<Forwarded, String> {
    forward_by("curl", request, (secret, staged, curls), client)
}

/// [`forward`] with `program` as curl, for the tests' stand-ins.
fn forward_by(
    program: &str,
    request: &Request,
    (secret, staged, curls): (&Secret, &Staged, &Curls),
    client: &mut impl Write,
) -> Result<Forwarded, String> {
    let mut curl = started(program, request, (secret, &staged.0))?;
    curls.started(&curl);
    let stdout = curl.stdout.take().ok_or("curl's stdout was not opened")?;
    let mut response = BufReader::new(stdout);
    let status = passed_head(&mut response, client)
        .map_err(|why| ended(&mut curl, (secret, curls), &why))?;
    let mut scan = Scan::default();
    let client_gone = !streamed(&mut response, client, &mut scan);
    if client_gone {
        let _ = curl.kill(); // the session stopped reading; curl's output goes nowhere
    }
    drop(response);
    let exit = curls
        .reaped(&mut curl)
        .map_err(|e| format!("waiting for curl: {e}"))?;
    let counted = match (client_gone, exit.success()) {
        (true, _) => Counted::Cut("the session stopped reading the response".into()),
        (false, false) => Counted::Cut(format!("curl ended with {exit} after the response head")),
        (false, true) => scan.counted(),
    };
    Ok(Forwarded { status, counted })
}

fn started(
    program: &str,
    request: &Request,
    (secret, staged): (&Secret, &Path),
) -> Result<Child, String> {
    let mut command = Command::new(program);
    // curl's environment is cleared (review of #476-#478, N5): no
    // SSLKEYLOGFILE to write the TLS secrets out, no proxy variable, no
    // CA bundle in place of the system's
    command.env_clear().env("PATH", CURL_PATH);
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
    // --globoff: the target is one request, never a pattern curl expands
    // (review of #476-#478, H1); --noproxy: the key goes to the API alone,
    // whatever proxy the environment names (L3)
    let mut args: Vec<String> = [
        "-q",
        "--silent",
        "--show-error",
        "--no-buffer",
        "--http1.1",
        "--globoff",
        "--noproxy",
        "*",
    ]
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

/// The response's body copied to `client` and read by `scan` until curl
/// ends it; whether the client took all of it.
fn streamed(response: &mut impl Read, client: &mut impl Write, scan: &mut Scan) -> bool {
    let mut buffer = [0; 16 * 1024];
    loop {
        let read = match response.read(&mut buffer) {
            Ok(0) => return true,
            Ok(read) => read,
            Err(error) => {
                eprintln!("api-build: reading curl's output failed: {error}");
                return true; // curl's exit code judges the transfer
            }
        };
        scan.read(&buffer[..read]);
        if client
            .write_all(&buffer[..read])
            .and_then(|()| client.flush())
            .is_err()
        {
            return false;
        }
    }
}

/// curl's failure, after `why`, its output guarded.
fn ended(curl: &mut Child, (secret, curls): (&Secret, &Curls), why: &str) -> String {
    let _ = curl.kill(); // it may have ended already
    let _ = curls.reaped(curl); // its stderr is read below
    let mut said = String::new();
    if let Some(mut stderr) = curl.stderr.take() {
        let _ = stderr.read_to_string(&mut said);
    }
    let said = secret
        .guard("curl's error", said.trim())
        .unwrap_or("(withheld: it held part of the key)");
    // curl's error echoes the target, which the session chose (M4)
    crate::api_refusals::escaped(&format!("{why}; curl said: {said}"))
}

#[cfg(test)]
#[path = "api_upstream_tests.rs"]
mod tests;
