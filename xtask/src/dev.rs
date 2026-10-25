//! `cargo xtask dev`: a save-to-reload loop on the phone. `trunk serve`
//! builds the web package (debug, for speed) and rebuilds it on every
//! save under the workspace, serving it on a local port with its reload
//! script injected; `tailscale serve` puts that port on the tailnet over
//! https on `[dev] https`, so the page on the phone reloads on save; the
//! release build keeps its own place (`cargo xtask serve`). Ctrl-C stops
//! trunk (it shares the terminal); that port's proxy then points at a
//! dead port until the next `cargo xtask dev`. With `--key
//! <provider>=<file>`, trunk proxies `/dev/keys.json` to a loopback
//! server that answers it from those files (`dev_keys_server`), so the
//! debug page can ask a model without a key typed in.

use std::ffi::OsStr;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Duration;

use crate::config::Config;
use crate::dev_keys::{self, DevKey};
use crate::dev_keys_server::{self, KEYS_PATH};
use crate::report::Reporter;
use crate::serve_config;
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
pub fn run(reporter: &Reporter, key_arguments: &[String]) -> ExitCode {
    match serve(reporter, key_arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            println!("FAIL dev: {message}");
            println!("     reproduce: cargo xtask dev (with the same --key arguments)");
            ExitCode::FAILURE
        }
    }
}

/// Every refusal before trunk starts, then the loop. `Trunk` kills its
/// process group when dropped, so every `?` after the spawn kills it.
fn serve(reporter: &Reporter, key_arguments: &[String]) -> Result<(), String> {
    let keys = checked_keys(key_arguments, &providers_for(key_arguments)?)?;
    let trunk = require_trunk()?;
    let tailscale = require_tailscale()?;
    let config = Config::load().map_err(|e| e.to_string())?;
    let port = port_from(&config)?;
    let mount = serve_config::dev(&config)?;
    check_port_free(port)?;
    let package = web_package()?;
    let crate_dir = package_dir(&package)?;
    let root = std::env::current_dir().map_err(|e| format!("no current directory: {e}"))?;
    let (notices, keys_port) = serve_keys(keys)?;
    let args = trunk_args(port, &root, keys_port);
    let mut trunk = Trunk::spawn(trunk_command(&trunk, &crate_dir, &args)?)?;
    trunk.wait_until_ready(port, reporter, READY_DEADLINE)?;
    start_serving(&tailscale, &Target::LocalPort(port), &mount)?;
    announce(&served_url(&tailscale, &mount)?, &package, port, &notices);
    trunk.follow(reporter)
}

/// The keys server started; its notice lines (never a key) and port.
fn serve_keys(keys: Vec<DevKey>) -> Result<(Vec<String>, u16), String> {
    let notices = keys.iter().map(dev_keys::notice).collect();
    Ok((notices, dev_keys_server::start(keys)?))
}

fn announce(url: &str, package: &str, port: u16, notices: &[String]) {
    println!(
        "ok   dev      {url} (trunk debug build of {package} on 127.0.0.1:{port}; reloads on save)"
    );
    println!("     Ctrl-C stops it; the release build stays where `cargo xtask serve` put it");
    for line in notices {
        println!("{line}");
    }
}

/// Every `--key` parsed and its file read once, so a bad one is refused
/// before anything starts. The keys read are dropped here.
pub fn checked_keys(arguments: &[String], providers: &[String]) -> Result<Vec<DevKey>, String> {
    let keys = arguments
        .iter()
        .map(|argument| dev_keys::parse(argument, providers))
        .collect::<Result<Vec<_>, _>>()?;
    for key in &keys {
        dev_keys::read(key)?;
    }
    refuse_repeats(&keys)?;
    Ok(keys)
}

/// The keyed providers when any `--key` is given; the words file is not
/// read otherwise.
fn providers_for(arguments: &[String]) -> Result<Vec<String>, String> {
    if arguments.is_empty() {
        return Ok(Vec::new());
    }
    keyed_providers(&Config::load().map_err(|e| e.to_string())?)
}

/// The providers the page's words file (`[dev] words`) gives a key.
///
/// # Errors
///
/// No `[dev] words`, or a file that cannot be read or names none.
pub fn keyed_providers(config: &Config) -> Result<Vec<String>, String> {
    let path = config
        .require("dev.words")
        .map_err(|e| format!("{e}; fix: set `[dev] words` to the page's words file, which says which providers take a key"))?;
    let words = std::fs::read_to_string(path).map_err(|e| {
        format!("{path} (`[dev] words` in .config/factory.toml) cannot be read: {e}")
    })?;
    dev_keys::keyed_providers(&words, path)
}

/// Refuse a provider given twice, naming both files (each has been
/// read, so each is a real file, not a key typed in its place).
fn refuse_repeats(keys: &[DevKey]) -> Result<(), String> {
    for (index, key) in keys.iter().enumerate() {
        if let Some(earlier) = keys[..index].iter().find(|k| k.provider == key.provider) {
            return Err(format!(
                "{} is given twice in --key, as {} and {}; fix: give each provider one --key",
                key.provider,
                earlier.file.display(),
                key.file.display()
            ));
        }
    }
    Ok(())
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
/// rebuilt on any save under `root`, into [`DEV_DIST`], with
/// `/dev/keys.json` proxied to the keys server on `keys_port`.
pub fn trunk_args(port: u16, root: &Path, keys_port: u16) -> Vec<String> {
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
        "--proxy-backend".into(),
        format!("http://127.0.0.1:{keys_port}{KEYS_PATH}"),
        "--proxy-rewrite".into(),
        KEYS_PATH.into(),
    ]
}

fn trunk_command(trunk: &Path, crate_dir: &Path, args: &[String]) -> Result<Command, String> {
    let program = trunk
        .to_str()
        .ok_or_else(|| format!("{} is not UTF-8", trunk.display()))?;
    let mut command = crate::priority::command(program)?;
    crate::sccache::apply(&mut command)?;
    command.args(args).current_dir(crate_dir);
    Ok(command)
}

#[cfg(test)]
#[path = "dev_tests.rs"]
mod tests;
