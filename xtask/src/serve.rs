//! `cargo xtask serve`: put `target/web` on the tailnet over https with
//! `tailscale serve` and print the URL, so a phone can open it.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Duration;

use crate::deadline::{Finished, run_with_deadline};
use crate::web::DIST;

/// How long `tailscale serve --bg` and `tailscale status` may take. Both
/// returned in under a second on the dev box on 2026-10-22; the margin
/// covers a daemon that is restarting. A hung daemon is a failure, not
/// a wait.
const TAILSCALE_DEADLINE: Duration = Duration::from_secs(30);

/// Serve the dist directory and print its URL.
pub fn run() -> ExitCode {
    match serve() {
        Ok(url) => {
            println!("ok   serve    {url}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            println!("FAIL serve: {message}");
            println!("     reproduce: cargo xtask serve");
            ExitCode::FAILURE
        }
    }
}

fn serve() -> Result<String, String> {
    let tailscale = find_on_path("tailscale").ok_or(
        "tailscale is not on PATH; install it from https://tailscale.com/download and log in",
    )?;
    let dist = std::env::current_dir()
        .map_err(|e| format!("no current directory: {e}"))?
        .join(DIST);
    if !dist.join("index.html").is_file() {
        return Err(format!(
            "{} has no index.html; run `cargo xtask web` first",
            dist.display()
        ));
    }
    start_serving(&tailscale, &dist)?;
    let status = tailscale_status(&tailscale)?;
    let name = dns_name(&status)?;
    Ok(format!("https://{name}/"))
}

/// The first `name` in `PATH`, as `which` would find it.
pub fn find_on_path(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|dir| dir.join(name))
            .find(|candidate| candidate.is_file())
    })
}

/// `sudo -n` never prompts: when sudo would ask for a password the
/// failure prints the exact command for the person to run themselves.
fn start_serving(tailscale: &Path, dist: &Path) -> Result<(), String> {
    let line = format!(
        "sudo {} serve --bg --https=443 {}",
        tailscale.display(),
        dist.display()
    );
    let mut command = Command::new("sudo");
    command
        .arg("-n")
        .arg(tailscale)
        .args(["serve", "--bg", "--https=443"])
        .arg(dist);
    match run_with_deadline(command, TAILSCALE_DEADLINE)? {
        Finished::Exited { success: true, .. } => Ok(()),
        Finished::Exited { output, .. } => Err(format!(
            "`{line}` failed:\n{output}fix: run that command yourself (sudo may want a password), then `cargo xtask serve` again"
        )),
        Finished::TimedOut => Err(format!(
            "`{line}` did not finish within {}s",
            TAILSCALE_DEADLINE.as_secs()
        )),
    }
}

fn tailscale_status(tailscale: &Path) -> Result<String, String> {
    let mut command = Command::new(tailscale);
    command.args(["status", "--json"]);
    match run_with_deadline(command, TAILSCALE_DEADLINE)? {
        Finished::Exited {
            success: true,
            output,
        } => Ok(output),
        Finished::Exited { output, .. } => {
            Err(format!("`tailscale status --json` failed:\n{output}"))
        }
        Finished::TimedOut => Err(format!(
            "`tailscale status --json` did not finish within {}s",
            TAILSCALE_DEADLINE.as_secs()
        )),
    }
}

/// `Self.DNSName` without its trailing dot. `Self` is null when this
/// machine is not logged in, and a peer's name must never stand in.
pub fn dns_name(status: &str) -> Result<String, String> {
    let json: serde_json::Value = serde_json::from_str(status)
        .map_err(|e| format!("`tailscale status --json` is not JSON: {e}"))?;
    let this_machine = match &json["Self"] {
        serde_json::Value::Object(fields) => fields,
        serde_json::Value::Null => {
            return Err("tailscale status has no Self: this machine is not logged in; run `sudo tailscale up`".into());
        }
        other => return Err(format!("tailscale status Self is {other}, not an object")),
    };
    this_machine["DNSName"]
        .as_str()
        .filter(|name| !name.is_empty())
        .map(|name| name.trim_end_matches('.').to_owned())
        .ok_or_else(|| "tailscale status Self has no DNSName; is MagicDNS on?".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dns_name_comes_from_self_without_the_trailing_dot() {
        // Given
        let status = r#"{"Peer":{"a":{"DNSName":"other.ts.net."}},"Self":{"ID":"x","DNSName":"debian.tail.ts.net.","Online":true}}"#;

        // When
        let name = dns_name(status).expect("has a name");

        // Then
        assert_eq!(name, "debian.tail.ts.net");
    }

    #[test]
    fn a_null_self_is_not_logged_in_and_a_peer_does_not_stand_in() {
        // Given
        let status = r#"{"Self":null,"Peer":{"a":{"DNSName":"other.ts.net."}}}"#;

        // When
        let error = dns_name(status).expect_err("not logged in");

        // Then
        assert!(error.contains("not logged in"), "{error}");
    }

    #[test]
    fn a_status_without_self_is_refused() {
        // Given
        let status = r#"{"Peer":{}}"#;

        // When
        let error = dns_name(status).expect_err("no Self");

        // Then
        assert!(error.contains("no Self"), "{error}");
    }

    #[test]
    fn text_that_is_not_json_is_refused() {
        // Given
        let status = "tailscale: command not found";

        // When
        let error = dns_name(status).expect_err("not JSON");

        // Then
        assert!(error.contains("not JSON"), "{error}");
    }

    #[test]
    fn find_on_path_finds_sh_and_not_a_made_up_name() {
        // When / Then
        assert!(find_on_path("sh").is_some());
        assert!(find_on_path("no-such-program-vapors").is_none());
    }
}
