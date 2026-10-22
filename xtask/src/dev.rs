//! `cargo xtask dev`: a save-to-reload loop on the phone. `trunk serve`
//! builds the web package (debug, for speed) and rebuilds it on every
//! save under the workspace, serving it on a local port with its reload
//! script injected; `tailscale serve` puts that port on the tailnet over
//! https, so the page on the phone reloads on save. The previous static
//! serve is replaced; `cargo xtask serve` puts it back. Ctrl-C stops
//! trunk (it shares the terminal); the tailscale proxy stays pointed at
//! the dead port until `cargo xtask serve`.

use std::ffi::OsStr;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::report::Reporter;
use crate::tailscale::{Target, find_in, require_tailscale, served_url, start_serving};
use crate::web::{package_dir, web_package};

/// The port trunk listens on when `[dev] port` is not set.
pub const DEFAULT_PORT: u16 = 8080;

/// Where trunk writes the dev build, relative to the workspace root;
/// apart from `target/web` so the static build is not overwritten.
pub const DEV_DIST: &str = "target/web-dev";

/// The line trunk prints once its server is up.
const READY_LINE: &str = "server listening at:";

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

fn serve(reporter: &Reporter) -> Result<(), String> {
    let trunk = require_trunk()?;
    let tailscale = require_tailscale()?;
    let config = Config::load().map_err(|e| e.to_string())?;
    let port = port_from(&config)?;
    check_port_free(port)?;
    let package = web_package()?;
    let crate_dir = package_dir(&package)?;
    let root = std::env::current_dir().map_err(|e| format!("no current directory: {e}"))?;
    let mut child = spawn_trunk(&trunk, &crate_dir, &trunk_args(port, &root))?;
    let lines = stream_lines(&mut child);
    wait_until_ready(&mut child, &lines, reporter, READY_DEADLINE)?;
    start_serving(&tailscale, &Target::LocalPort(port))?;
    let url = served_url(&tailscale)?;
    println!(
        "ok   dev      {url} (trunk debug build of {package} on 127.0.0.1:{port}; reloads on save)"
    );
    println!("     Ctrl-C stops it; `cargo xtask serve` puts the static build back on the tailnet");
    follow(&mut child, &lines, reporter)
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

/// `dev.port` from the settings, or [`DEFAULT_PORT`].
pub fn port_from(config: &Config) -> Result<u16, String> {
    match config.get("dev.port") {
        None => Ok(DEFAULT_PORT),
        Some(text) => text.parse().map_err(|_| {
            format!(
                ".config/factory.toml: [dev] port = \"{text}\" is not a port number (1..=65535)"
            )
        }),
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

fn spawn_trunk(trunk: &Path, crate_dir: &Path, args: &[String]) -> Result<Child, String> {
    Command::new(trunk)
        .args(args)
        .current_dir(crate_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start {}: {e}", trunk.display()))
}

/// Both of trunk's streams, line by line, on one channel.
fn stream_lines(child: &mut Child) -> Receiver<String> {
    let (sender, receiver) = mpsc::channel();
    for stream in [
        child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
        child
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let sender = sender.clone();
        thread::spawn(move || {
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
    }
    receiver
}

/// Print trunk's lines until its ready line, within `deadline`
/// ([`READY_DEADLINE`] in use; the tests pass a short one).
fn wait_until_ready(
    child: &mut Child,
    lines: &Receiver<String>,
    reporter: &Reporter,
    deadline: Duration,
) -> Result<(), String> {
    let started = Instant::now();
    loop {
        match lines.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => {
                reporter.stream("trunk", &line);
                if line.contains(READY_LINE) {
                    return Ok(());
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                return Err(exited_early(child));
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
        if started.elapsed() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "trunk did not print `{READY_LINE}` within {}s; fix: run `trunk serve` in the package directory to see why",
                deadline.as_secs()
            ));
        }
    }
}

fn exited_early(child: &mut Child) -> String {
    let status = child
        .wait()
        .map_or_else(|e| e.to_string(), |s| s.to_string());
    format!("trunk exited before its server was up ({status}); its lines are above")
}

/// Stream rebuild lines until trunk exits; a non-zero exit is a failure.
fn follow(child: &mut Child, lines: &Receiver<String>, reporter: &Reporter) -> Result<(), String> {
    while let Ok(line) = lines.recv() {
        reporter.stream("trunk", &line);
    }
    let status = child.wait().map_err(|e| format!("trunk: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("trunk exited with {status}"))
    }
}

#[cfg(test)]
#[path = "dev_tests.rs"]
mod tests;
