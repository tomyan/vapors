//! The release page's server: static-web-server, the binary pinned and
//! verified by hash (`[downloads]`, `download_install.rs`), as a user
//! systemd unit on 127.0.0.1 behind `tailscale serve` (`serve_unit.rs`).
//! It serves the copy publish just made, each file's brotli or gzip copy
//! by the request's `Accept-Encoding`, and cache headers. The unit
//! starts at boot only with lingering on, so serve refuses without it.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::download_install::verified;
use crate::downloads::pinned;
use crate::serve_listen::wait_for_unit;
use crate::serve_unit::{Paths, config_text, unit_name, unit_text};

/// The tool's name in `[downloads]`.
const SERVER: &str = "static-web-server";

/// Write the config and the unit, (re)start it, and return its name once
/// it answers on 127.0.0.1:`port` as the port's owner.
///
/// # Errors
///
/// No lingering, the binary unverified, a path the unit cannot carry,
/// systemctl failing, or the port not the unit's, naming the fix.
pub fn start(dir: &Path, port: u16, home: &Path) -> Result<String, String> {
    lingering()?;
    let server = verified(&pinned(SERVER)?, home)?;
    let copy = std::fs::canonicalize(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let config = PathBuf::from(format!("{}.sws.toml", dir.display()));
    let paths = Paths {
        server: &server,
        dir,
        copy: &copy,
        config: &config,
    };
    let unit = unit_text(&paths, port)?;
    write(&config, &config_text())?;
    let name = unit_name(dir);
    let units = home.join(".config/systemd/user");
    std::fs::create_dir_all(&units).map_err(|e| format!("{}: {e}", units.display()))?;
    write(&units.join(&name), &unit)?;
    for args in [
        vec!["daemon-reload"],
        vec!["enable", &name],
        vec!["restart", &name],
    ] {
        systemctl(&args)?;
    }
    wait_for_unit(&name, port)?;
    Ok(name)
}

/// A user unit runs at boot, and after logout, only with lingering on.
fn lingering() -> Result<(), String> {
    let user = std::env::var("USER").map_err(|_| "USER is unset; fix: set it".to_owned())?;
    let output = Command::new("loginctl")
        .args(["show-user", &user, "-p", "Linger", "--value"])
        .output()
        .map_err(|e| format!("loginctl: {e}; fix: run where systemd-logind runs"))?;
    if String::from_utf8_lossy(&output.stdout).trim() == "yes" {
        return Ok(());
    }
    Err(format!(
        "{user} has no lingering, so the server would stop at logout and not start at boot; fix: loginctl enable-linger {user}"
    ))
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

fn systemctl(args: &[&str]) -> Result<(), String> {
    let output = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .map_err(|e| format!("systemctl --user: {e}; fix: run where a user systemd is running"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "systemctl --user {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}
