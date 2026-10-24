//! `tailscale serve` as the factory uses it: a directory or a local http
//! port put on the tailnet over https, and the machine's DNS name for
//! the URL. Shared by `serve` (the static build) and `dev` (trunk's
//! server behind the same proxy).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::deadline::{Finished, run_with_deadline};

/// How long `tailscale serve --bg` and `tailscale status` may take. Both
/// returned in under a second on the dev box on 2026-10-22; the margin
/// covers a daemon that is restarting. A hung daemon is a failure, not
/// a wait.
pub const TAILSCALE_DEADLINE: Duration = Duration::from_secs(30);

/// The first `name` in `path` (a `PATH`-shaped value), for lookups the
/// tests run against a PATH of their own.
pub fn find_in(path: Option<&OsStr>, name: &str) -> Option<PathBuf> {
    path.and_then(|path| {
        std::env::split_paths(path)
            .map(|dir| dir.join(name))
            .find(|candidate| candidate.is_file())
    })
}

/// The tailscale binary, or the refusal naming the fix.
pub fn require_tailscale() -> Result<PathBuf, String> {
    require_tailscale_in(std::env::var_os("PATH").as_deref())
}

/// [`require_tailscale`] against `path`.
pub fn require_tailscale_in(path: Option<&OsStr>) -> Result<PathBuf, String> {
    find_in(path, "tailscale").ok_or_else(|| {
        "tailscale is not on PATH; install it from https://tailscale.com/download and log in"
            .to_owned()
    })
}

/// What `tailscale serve` puts behind https on the tailnet.
pub enum Target<'a> {
    /// A directory of static files.
    Directory(&'a Path),
    /// A local http server, as `http://127.0.0.1:<port>`.
    LocalPort(u16),
}

impl Target<'_> {
    fn argument(&self) -> String {
        match self {
            Target::Directory(dir) => dir.display().to_string(),
            Target::LocalPort(port) => format!("http://127.0.0.1:{port}"),
        }
    }
}

/// `sudo -n tailscale serve --bg --https=443 <target>`. `sudo -n` never
/// prompts: when sudo would ask for a password the failure prints the
/// exact command for the person to run themselves. The previous serve
/// target, if any, is replaced.
pub fn start_serving(tailscale: &Path, target: &Target<'_>) -> Result<(), String> {
    let argument = target.argument();
    let line = format!(
        "sudo {} serve --bg --https=443 {argument}",
        tailscale.display()
    );
    let mut command = Command::new("sudo");
    command
        .arg("-n")
        .arg(tailscale)
        .args(["serve", "--bg", "--https=443"])
        .arg(&argument);
    match run_with_deadline(command, TAILSCALE_DEADLINE)? {
        Finished::Exited { success: true, .. } => Ok(()),
        Finished::Exited { output, .. } => Err(format!(
            "`{line}` failed:\n{output}fix: run that command yourself (sudo may want a password), then the xtask command again"
        )),
        Finished::TimedOut { .. } => Err(format!(
            "`{line}` did not finish within {}s",
            TAILSCALE_DEADLINE.as_secs()
        )),
    }
}

/// `https://<Self.DNSName>/` for this machine.
pub fn served_url(tailscale: &Path) -> Result<String, String> {
    let status = tailscale_status(tailscale)?;
    let name = dns_name(&status)?;
    Ok(format!("https://{name}/"))
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
        Finished::TimedOut { .. } => Err(format!(
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
#[path = "tailscale_tests.rs"]
mod tests;
