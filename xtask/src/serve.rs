//! `cargo xtask serve`: put `target/web` on the tailnet over https with
//! `tailscale serve` and print the URL, so a phone can open it.

use std::path::Path;
use std::process::{Command, ExitCode};

use crate::web::DIST;

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
    let dist = std::env::current_dir()
        .map_err(|e| format!("no current directory: {e}"))?
        .join(DIST);
    if !dist.join("index.html").is_file() {
        return Err(format!(
            "{} has no index.html; run `cargo xtask web` first",
            dist.display()
        ));
    }
    start_serving(&dist)?;
    let status = tailscale_status()?;
    let name = dns_name(&status)?;
    Ok(format!("https://{name}/"))
}

fn start_serving(dist: &Path) -> Result<(), String> {
    let output = Command::new("sudo")
        .args(["tailscale", "serve", "--bg", "--https=443"])
        .arg(dist)
        .output()
        .map_err(|e| format!("could not run sudo: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "`sudo tailscale serve --bg --https=443 {}` failed: {}{}fix: install tailscale and join a tailnet, or run that command by hand to see its prompt",
        dist.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn tailscale_status() -> Result<String, String> {
    let output = Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .map_err(|e| {
            format!("could not run tailscale: {e}; install it from https://tailscale.com/download")
        })?;
    if !output.status.success() {
        return Err(format!(
            "`tailscale status --json` failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// `Self.DNSName` without its trailing dot, read from the status JSON
/// without a JSON parser: the first `"DNSName"` after `"Self"`.
pub fn dns_name(status: &str) -> Result<String, String> {
    let after_self = status
        .split_once("\"Self\"")
        .map(|(_, rest)| rest)
        .ok_or("tailscale status has no \"Self\" entry; is this machine logged in?")?;
    let after_key = after_self
        .split_once("\"DNSName\"")
        .map(|(_, rest)| rest)
        .ok_or("tailscale status has no \"DNSName\" under \"Self\"")?;
    let value = after_key
        .split_once('"')
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(value, _)| value)
        .ok_or("\"DNSName\" has no string value")?;
    Ok(value.trim_end_matches('.').to_owned())
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
    fn a_status_without_self_is_refused() {
        // Given
        let status = r#"{"Peer":{}}"#;

        // When
        let error = dns_name(status).expect_err("no Self");

        // Then
        assert!(error.contains("\"Self\""));
    }
}
