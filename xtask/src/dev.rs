//! `cargo xtask dev`: a save-to-reload loop on the phone. `trunk serve`
//! builds the web package (debug, for speed) and rebuilds it on every
//! save under the workspace, serving it on a local port with its reload
//! script injected; `tailscale serve` puts that port on the tailnet over
//! https, so the page on the phone reloads on save. The previous static
//! serve is replaced; `cargo xtask serve` puts it back. Ctrl-C stops
//! trunk (it shares the terminal); the tailscale proxy stays pointed at
//! the dead port until `cargo xtask serve`.

use std::ffi::OsStr;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Duration;

use crate::config::Config;
use crate::report::Reporter;
use crate::tailscale::{Target, find_in, require_tailscale, served_url, start_serving};
use crate::trunk::Trunk;
use crate::web::{package_dir, web_package};

/// The port trunk listens on when `[dev] port` is not set.
pub const DEFAULT_PORT: u16 = 8080;

/// Where trunk writes the dev build, relative to the workspace root;
/// apart from `target/web` so the static build is not overwritten.
pub const DEV_DIST: &str = "target/web-dev";

/// How long the first build may take before the server is up. The
/// demo's debug build with its dependencies already compiled measured
/// 30 s on the dev box on 2026-10-22; with every dependency cold it is
/// a few minutes, and CI runners are slower, hence the margin.
pub const READY_DEADLINE: Duration = Duration::from_secs(600);

/// Run the loop until trunk exits.
pub fn run(reporter: &Reporter) -> ExitCode {
    match serve(reporter) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            println!("FAIL dev: {message}");
            println!("     reproduce: cargo xtask dev");
            ExitCode::FAILURE
        }
    }
}

/// Every refusal before trunk starts, then the loop. `Trunk` kills its
/// process group when dropped, so every `?` after the spawn kills it.
fn serve(reporter: &Reporter) -> Result<(), String> {
    let trunk = require_trunk()?;
    let tailscale = require_tailscale()?;
    let config = Config::load().map_err(|e| e.to_string())?;
    let port = port_from(&config)?;
    check_port_free(port)?;
    let package = web_package()?;
    let crate_dir = package_dir(&package)?;
    let root = std::env::current_dir().map_err(|e| format!("no current directory: {e}"))?;
    let mut trunk = Trunk::spawn(trunk_command(&trunk, &crate_dir, &trunk_args(port, &root)))?;
    trunk.wait_until_ready(port, reporter, READY_DEADLINE)?;
    start_serving(&tailscale, &Target::LocalPort(port))?;
    let url = served_url(&tailscale)?;
    println!(
        "ok   dev      {url} (trunk debug build of {package} on 127.0.0.1:{port}; reloads on save)"
    );
    println!("     Ctrl-C stops it; `cargo xtask serve` puts the static build back on the tailnet");
    trunk.follow(reporter)
}

/// The trunk binary, or the refusal naming the fix.
pub fn require_trunk() -> Result<PathBuf, String> {
    require_trunk_in(std::env::var_os("PATH").as_deref())
}

/// [`require_trunk`] against `path`.
pub fn require_trunk_in(path: Option<&OsStr>) -> Result<PathBuf, String> {
    find_in(path, "trunk").ok_or_else(|| {
        "trunk is not on PATH; fix: `cargo xtask setup` installs the pinned version".to_owned()
    })
}

/// `dev.port` from the settings, or [`DEFAULT_PORT`]. Zero is not a
/// port something can listen on.
pub fn port_from(config: &Config) -> Result<u16, String> {
    let Some(text) = config.get("dev.port") else {
        return Ok(DEFAULT_PORT);
    };
    match text.parse::<u16>() {
        Ok(port) if port > 0 => Ok(port),
        _ => Err(format!(
            ".config/factory.toml: [dev] port = \"{text}\" is not a port number (1..=65535)"
        )),
    }
}

/// Refuse a port something else is listening on, naming it.
pub fn check_port_free(port: u16) -> Result<(), String> {
    TcpListener::bind(("127.0.0.1", port)).map(drop).map_err(|e| {
        format!(
            "127.0.0.1:{port} is in use ({e}); fix: stop what listens there, or set `[dev] port = \"...\"` in .config/factory.toml"
        )
    })
}

/// trunk's arguments: a debug build served on the loopback port,
/// rebuilt on any save under `root`, into [`DEV_DIST`].
pub fn trunk_args(port: u16, root: &Path) -> Vec<String> {
    vec![
        "serve".into(),
        "--address".into(),
        "127.0.0.1".into(),
        "--port".into(),
        port.to_string(),
        "--watch".into(),
        root.display().to_string(),
        "--dist".into(),
        root.join(DEV_DIST).display().to_string(),
    ]
}

fn trunk_command(trunk: &Path, crate_dir: &Path, args: &[String]) -> Command {
    let mut command = Command::new(trunk);
    command.args(args).current_dir(crate_dir);
    command
}

#[cfg(test)]
#[path = "dev_tests.rs"]
mod tests;
