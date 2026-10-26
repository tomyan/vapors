//! The one request a live review makes: curl POSTs the body file to the
//! endpoint, reading the Authorization header from its stdin so the key
//! is never in a command line, and writes the response to a file. The
//! run is judged by curl's exit code and the HTTP status, not by a
//! summary; everything shown from it passes the key guard first.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use serde_json::Value;

use crate::ux_secret::Secret;

/// The whole request, connection to last byte: chosen at 300 s for a
/// reasoning model with ten frames before any live run; replaced by
/// three times the slowest of the first live runs (U4 slice 3).
pub const DEADLINE: Duration = Duration::from_secs(300);

/// curl's arguments for the request: never the key.
pub fn arguments(endpoint: &str, body: &Path, response: &Path, deadline: Duration) -> Vec<String> {
    let path = |p: &Path| p.display().to_string();
    vec![
        // -q first: a ~/.curlrc (with --verbose or --trace) is not read.
        "-q".into(),
        "--silent".into(),
        "--show-error".into(),
        "--connect-timeout".into(),
        "20".into(),
        "--max-time".into(),
        deadline.as_secs().to_string(),
        "--request".into(),
        "POST".into(),
        "--header".into(),
        "Content-Type: application/json".into(),
        "--header".into(),
        "@-".into(),
        "--data-binary".into(),
        format!("@{}", path(body)),
        "--output".into(),
        path(response),
        "--write-out".into(),
        "%{http_code}".into(),
        endpoint.to_owned(),
    ]
}

/// POST `body` to `endpoint` and return the response, parsed.
///
/// # Errors
/// curl missing or failing, a status other than 200, or a response that
/// is not JSON; each message has passed the key guard.
pub fn post(
    endpoint: &str,
    secret: &Secret,
    body: &Path,
    response: &Path,
    deadline: Duration,
) -> Result<Value, String> {
    // A response left by an earlier run must not be read as this one's.
    match std::fs::remove_file(response) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            return Err(format!("{}: {e}; fix: remove it", response.display()));
        }
        _ => {}
    }
    let mut curl = Command::new("curl");
    secret.unset_in(&mut curl);
    let mut child = curl
        .args(arguments(endpoint, body, response, deadline))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not run curl: {e}; fix: install curl"))?;
    let header = secret.header();
    child
        .stdin
        .take()
        .ok_or("curl's stdin was not opened")?
        .write_all(header.as_bytes())
        .map_err(|e| format!("writing curl's header: {e}"))?;
    let output = child
        .wait_with_output()
        .map_err(|e| format!("waiting for curl: {e}"))?;
    let said = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        let said = secret.guard("curl's error", said.trim())?;
        return Err(forget(
            response,
            &format!("curl failed ({}): {said}", output.status),
        ));
    }
    answer(secret, &String::from_utf8_lossy(&output.stdout), response)
}

/// The response, when the status is 200 and the body is JSON.
fn answer(secret: &Secret, status: &str, response: &Path) -> Result<Value, String> {
    let text =
        std::fs::read_to_string(response).map_err(|e| format!("{}: {e}", response.display()))?;
    let text = match secret.guard("the response", &text) {
        Ok(text) => text,
        Err(why) => return Err(forget(response, &why)),
    };
    if status != "200" {
        let head: String = text.chars().take(600).collect();
        return Err(forget(
            response,
            &format!("the endpoint answered HTTP {status}: {head}"),
        ));
    }
    serde_json::from_str(text)
        .map_err(|e| forget(response, &format!("the response is not JSON: {e}")))
}

/// A failed run's response is deleted, so no file keeps what the guard
/// refused, or a partial or error body: only a good answer stays. A file
/// curl never wrote is fine.
fn forget(response: &Path, why: &str) -> String {
    match std::fs::remove_file(response) {
        Ok(()) => format!("{why}; {} was deleted", response.display()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => why.to_owned(),
        Err(e) => format!(
            "{why}; {} could not be deleted ({e}); fix: delete it by hand",
            response.display()
        ),
    }
}

#[cfg(test)]
#[path = "ux_call_tests.rs"]
mod tests;
