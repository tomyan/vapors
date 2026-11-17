//! `cargo xtask api-fences <name>`: makes builder `<name>`'s clone, starts
//! the harness server, and proves the sandbox's fences and its way in
//! (`api_fences.rs`), as `api-build` will before every session. The
//! builder's directories are removed afterwards.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crate::api_fences::{Escapes, Ran, probes, prove, ran};
use crate::api_paths::{Builder, KEY_FILE};
use crate::api_sandbox::{BIN, Host, RELAY_PORT, arguments};

/// Proves builder `name`'s fences.
pub fn run(name: &str) -> ExitCode {
    match home()
        .and_then(|home| Builder::new(name, &home))
        .and_then(|builder| made_and_proven(&builder))
    {
        Ok(lines) => {
            for line in &lines {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        Err(why) => {
            println!("FAIL api-fences: {why}");
            ExitCode::FAILURE
        }
    }
}

/// The builder made, its fences proven, and its directories removed;
/// a builder already there is refused and left as it is.
fn made_and_proven(builder: &Builder) -> Result<Vec<String>, String> {
    let (main, objects) = crate::api_clone::main_repository()?;
    crate::api_clone::make(builder, &main, &format!("api/{}", builder.name))?;
    let result = proven(builder, objects);
    removed(builder);
    result
}

fn proven(builder: &Builder, objects: PathBuf) -> Result<Vec<String>, String> {
    let host = host(objects)?;
    let routes = std::sync::Arc::new(crate::api_server::Routes::default());
    crate::api_server::serve(&builder.socket(), routes)?;
    proven_on(builder, &host)
}

/// Builder's fences and way in proven, the harness server already on its
/// socket; a line for each.
///
/// # Errors
///
/// A fence broken, a probe that cannot show its escape, or the way in or
/// the toolchain not working inside.
pub fn proven_on(builder: &Builder, host: &Host) -> Result<Vec<String>, String> {
    let inside = |script: &str| inside(builder, host, script);
    let key_file = builder.home.join(KEY_FILE);
    let escapes = Escapes {
        hidden: builder.root().join("escape-probe"),
        read_only: &host.objects,
        key_file: &key_file,
        clone_dir: &builder.clone_dir(),
    };
    let mut lines = prove(&probes(&escapes), &inside)?;
    lines.push(way_in(&inside)?);
    lines.push(toolchain(&inside)?);
    Ok(lines)
}

/// The paths outside the sandbox reads.
///
/// # Errors
///
/// This binary's path or the CLI not found.
pub fn host(objects: PathBuf) -> Result<Host, String> {
    let xtask = std::env::current_exe().map_err(|e| format!("finding this xtask binary: {e}"))?;
    Ok(Host {
        xtask,
        claude: claude()?,
        objects,
        dri: Path::new("/dev/dri").exists(),
    })
}

/// The CLI's binary, its symlinks resolved.
fn claude() -> Result<PathBuf, String> {
    let link = home()?.join(".local/bin/claude");
    std::fs::canonicalize(&link).map_err(|e| {
        format!(
            "the claude CLI is not at {} ({e}); fix: install it (claude.com/claude-code)",
            link.display()
        )
    })
}

/// `script` run by bash inside builder's sandbox, under the relay.
fn inside(builder: &Builder, host: &Host, script: &str) -> Result<Ran, String> {
    ran(sandboxed(builder, host, &["bash", "-c", script]))
}

/// `command` to run inside builder's sandbox, under the relay.
pub fn sandboxed(builder: &Builder, host: &Host, command: &[&str]) -> Command {
    let mut bwrap = Command::new("bwrap");
    bwrap.args(arguments(builder, host, RELAY_PORT));
    bwrap.arg(Path::new(BIN).join("xtask"));
    let port = RELAY_PORT.to_string();
    bwrap.args(["api-relay-inner", "--port", &port, "--socket"]);
    bwrap.arg(builder.socket());
    bwrap.arg("--").args(command);
    bwrap
}

/// The way in: a request through the relay reaches the harness server.
fn way_in(inside: &dyn Fn(&str) -> Result<Ran, String>) -> Result<String, String> {
    let ping = format!(
        "curl -q -sS --max-time 15 http://127.0.0.1:{RELAY_PORT}{}",
        crate::api_server::PING
    );
    let answer = inside(&ping)?;
    if answer.success && answer.said == "pong" {
        return Ok("way in: the relay reached the harness server".into());
    }
    Err(format!(
        "the way in is shut: `{ping}` inside the sandbox said {:?}; the relay is api_inner.rs, \
         the server api_server.rs",
        answer.said
    ))
}

/// The toolchain works inside: cargo resolves the workspace offline from
/// the read-only registry.
fn toolchain(inside: &dyn Fn(&str) -> Result<Ran, String>) -> Result<String, String> {
    let resolve = "cargo metadata --offline --format-version 1 > /dev/null";
    let answer = inside(resolve)?;
    if answer.success {
        return Ok("toolchain: cargo resolves the workspace offline".into());
    }
    Err(format!(
        "the toolchain does not work inside: `{resolve}` said {:?}; the mounts are \
         api_sandbox.rs",
        answer.said
    ))
}

fn removed(builder: &Builder) {
    for directory in [builder.root(), builder.target()] {
        if let Err(error) = std::fs::remove_dir_all(&directory) {
            println!(
                "api-fences: could not remove {}: {error}",
                directory.display()
            );
        }
    }
}

/// The home directory.
///
/// # Errors
///
/// `HOME` unset.
pub fn home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set: the builder's directories are under it".to_owned())
}
