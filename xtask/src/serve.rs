//! `cargo xtask serve`: the release build of main on the tailnet, where
//! `[serve]` in `.config/factory.toml` says (`serve_config.rs`). From a
//! checkout it fetches `origin/main`, refuses another commit or a
//! changed tree (`serve_git.rs`), builds the page itself so the build is
//! that commit's, runs `[serve] check` on it within its deadline
//! (`serve_check.rs`; touche's `check_dist` refuses, among the rest, any
//! file requesting the dev server's keys), publishes a copy under the
//! home directory (`serve_publish.rs`), starts static-web-server on it
//! at 127.0.0.1 (`serve_static.rs`), and mounts that, refusing to replace
//! another target unless `--replace` (`serve_mount.rs`). Then it fetches
//! `/` and the wasm through the tailnet URL and fails unless they come
//! compressed, with their cache headers (`serve_verify.rs`).

#[path = "serve_check.rs"]
mod check;
#[path = "serve_git.rs"]
mod git;
#[path = "serve_publish.rs"]
mod publish;
#[path = "serve_static.rs"]
mod static_server;
#[path = "serve_verify.rs"]
mod verify;

use std::path::Path;
use std::process::ExitCode;

use crate::config::Config;
use crate::serve_config::{self, Serve};
use crate::tailscale::{Target, mount, require_tailscale};
use crate::web::{Build, DIST, build_page};

/// Serve the release build of main and print its URL.
pub fn run(replace: bool) -> ExitCode {
    match serve(replace) {
        Ok((url, checked)) => {
            println!("ok   serve    {url} (the release build of main; {checked})");
            ExitCode::SUCCESS
        }
        Err(message) => {
            println!("FAIL serve: {message}");
            println!("     reproduce: cargo xtask serve");
            ExitCode::FAILURE
        }
    }
}

fn serve(replace: bool) -> Result<(String, String), String> {
    let tailscale = require_tailscale()?;
    let settings = settings()?;
    let root = std::env::current_dir().map_err(|e| format!("no current directory: {e}"))?;
    let here = root.display().to_string();
    git::fetch_main(&here)?;
    git::require_main(&here)?;
    build_page(&Build::of(false))?;
    check::run_check(&settings.check, settings.check_deadline)?;
    publish::publish(&root.join(DIST), &settings.dir)?;
    static_server::start(&settings.dir, settings.port, &home()?)?;
    let url = mount(
        &tailscale,
        &Target::LocalPort(settings.port),
        &settings.mount,
        replace,
    )?;
    let checked = verify::verify(&url, &settings.dir)?;
    Ok((url, checked))
}

fn home() -> Result<std::path::PathBuf, String> {
    std::env::var_os("HOME")
        .map(Into::into)
        .ok_or_else(|| "HOME is unset; fix: set it".to_owned())
}

fn settings() -> Result<Serve, String> {
    let home = std::env::var_os("HOME").ok_or("HOME is unset; fix: set it")?;
    let config = Config::load().map_err(|e| e.to_string())?;
    serve_config::serve(&config, Path::new(&home))
}
