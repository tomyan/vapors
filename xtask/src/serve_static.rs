//! The release page's server: static-web-server on 127.0.0.1 only, as a
//! user systemd unit, behind `tailscale serve`. It serves the published
//! copy (a symlink swapped per publish, resolved per request), each file's
//! brotli or gzip copy by the request's `Accept-Encoding`, and cache
//! headers: trunk's hashed files are immutable for a year, everything
//! else (index.html, version.json, the page's `?v=` files) `no-cache`, so
//! a phone asks again and gets 304 or the new build.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// trunk's hashed names: `<name>-<16 hex>.js` and `<name>-<16 hex>_bg.wasm`.
const HEX: &str = "[0-9a-f]";

/// How long the unit may take to report active. static-web-server
/// started in under 50 ms on the dev box (2026-10-26); systemd's restart
/// adds its own. 10 s is far past both.
const ACTIVE_DEADLINE: Duration = Duration::from_secs(10);

/// static-web-server's settings for `dir`.
pub fn config_text() -> String {
    let hashed = format!("**/*-{}{{_bg.wasm,.js}}", HEX.repeat(16));
    format!(
        "[general]\ncache-control-headers = false\ncompression = true\ncompression-static = true\n\n[advanced]\n\n[[advanced.headers]]\nsource = \"**/*\"\n[advanced.headers.headers]\nCache-Control = \"no-cache\"\n\n[[advanced.headers]]\nsource = \"{hashed}\"\n[advanced.headers.headers]\nCache-Control = \"public, max-age=31536000, immutable\"\n"
    )
}

/// The unit that runs `server` on `port` for `dir` with `config`.
pub fn unit_text(server: &Path, dir: &Path, port: u16, config: &Path) -> String {
    format!(
        "[Unit]\nDescription=cargo xtask serve: {dir} on 127.0.0.1:{port}\n\n[Service]\nExecStart={server} --host 127.0.0.1 --port {port} --root {dir} --use-relative-root --config-file {config}\nRestart=on-failure\n\n[Install]\nWantedBy=default.target\n",
        dir = dir.display(),
        server = server.display(),
        config = config.display()
    )
}

/// The unit's name for `dir`: `xtask-serve-<dir's name>.service`.
pub fn unit_name(dir: &Path) -> String {
    let name = dir
        .file_name()
        .map_or_else(|| "page".into(), |n| n.to_string_lossy().into_owned());
    format!("xtask-serve-{name}.service")
}

/// Write the config and the unit, and (re)start it; `Ok` once active.
///
/// # Errors
///
/// A file unwritten, systemctl failing, or the unit not active within
/// [`ACTIVE_DEADLINE`], naming the fix.
pub fn start(dir: &Path, port: u16, home: &Path) -> Result<String, String> {
    let server = located("static-web-server")?;
    let config = PathBuf::from(format!("{}.sws.toml", dir.display()));
    write(&config, &config_text())?;
    let name = unit_name(dir);
    let units = home.join(".config/systemd/user");
    std::fs::create_dir_all(&units).map_err(|e| format!("{}: {e}", units.display()))?;
    write(&units.join(&name), &unit_text(&server, dir, port, &config))?;
    systemctl(&["daemon-reload"])?;
    systemctl(&["enable", &name])?;
    systemctl(&["restart", &name])?;
    active(&name)?;
    Ok(name)
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

fn located(binary: &str) -> Result<PathBuf, String> {
    let output = Command::new("sh")
        .args(["-c", &format!("command -v {binary}")])
        .output()
        .map_err(|e| format!("sh: {e}"))?;
    let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if path.is_empty() {
        return Err(format!("{binary} is not on PATH; fix: cargo xtask setup"));
    }
    Ok(PathBuf::from(path))
}

fn systemctl(args: &[&str]) -> Result<String, String> {
    let output = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .map_err(|e| format!("systemctl --user: {e}; fix: run where a user systemd is running (loginctl enable-linger)"))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned());
    }
    Err(format!(
        "systemctl --user {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

fn active(name: &str) -> Result<(), String> {
    let started = Instant::now();
    while started.elapsed() < ACTIVE_DEADLINE {
        if systemctl(&["is-active", name]).is_ok_and(|s| s == "active") {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(format!(
        "{name} was not active within {} s; fix: journalctl --user -u {name}",
        ACTIVE_DEADLINE.as_secs()
    ))
}

#[cfg(test)]
#[path = "serve_static_tests.rs"]
mod tests;
