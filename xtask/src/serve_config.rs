//! Where `cargo xtask serve` and `cargo xtask dev` put a page on the
//! tailnet, from `.config/factory.toml`: `[serve]` for the release build
//! (`dir` under the home directory, `https`, `path`, and `check`, the
//! command that must pass on `target/web` first, within `check_seconds`)
//! and `[dev] https` for trunk's debug page, at `/` of its own port.

use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use crate::config::Config;

/// An https port and a path on it, as `tailscale serve` mounts a target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    pub https: u16,
    /// Starts and ends with `/`.
    pub path: String,
}

/// What `[serve]` says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Serve {
    /// The served copy, which outlives any worktree.
    pub dir: PathBuf,
    pub mount: Mount,
    /// The loopback port static-web-server listens on, behind the mount.
    pub port: u16,
    /// A shell line run in the checkout; the build is served only when
    /// it exits 0 within `check_deadline`.
    pub check: String,
    pub check_deadline: Duration,
}

/// `[serve]`, with `dir` under `home`.
///
/// # Errors
///
/// A key missing or malformed, naming it and the fix.
pub fn serve(config: &Config, home: &Path) -> Result<Serve, String> {
    Ok(Serve {
        dir: home.join(served_dir(required(config, "serve.dir")?)?),
        mount: Mount {
            https: port(config, "serve.https")?,
            path: path(config)?,
        },
        port: port(config, "serve.port")?,
        check: required(config, "serve.check")?.to_owned(),
        check_deadline: Duration::from_secs(seconds(config, "serve.check_seconds")?),
    })
}

/// A relative path of plain names under the home directory: no `.`,
/// `..`, empty part or root, so publish can never be pointed at the
/// home directory itself or above it.
fn served_dir(dir: &str) -> Result<&str, String> {
    let plain = !dir.is_empty()
        && !dir.split('/').any(str::is_empty)
        && Path::new(dir)
            .components()
            .all(|c| matches!(c, Component::Normal(_)));
    if plain {
        Ok(dir)
    } else {
        Err(format!(
            "serve.dir = \"{dir}\" must be plain names under the home directory, such as srv/<name>; fix: .config/factory.toml"
        ))
    }
}

fn seconds(config: &Config, key: &str) -> Result<u64, String> {
    let text = required(config, key)?;
    text.parse::<u64>().ok().filter(|s| *s > 0).ok_or_else(|| {
        format!(
            "{key} = \"{text}\" is not a whole number of seconds above 0; fix: .config/factory.toml"
        )
    })
}

/// `[dev] https`: trunk's debug page at `/` of that port.
///
/// # Errors
///
/// The key missing or not a port.
pub fn dev(config: &Config) -> Result<Mount, String> {
    Ok(Mount {
        https: port(config, "dev.https")?,
        path: "/".into(),
    })
}

fn required<'a>(config: &'a Config, key: &str) -> Result<&'a str, String> {
    config
        .require(key)
        .map_err(|e| format!("{e}; fix: set it in .config/factory.toml"))
}

fn port(config: &Config, key: &str) -> Result<u16, String> {
    let text = required(config, key)?;
    match text.parse::<u16>() {
        Ok(port) if port > 0 => Ok(port),
        _ => Err(format!(
            "{key} = \"{text}\" is not a port number (1..=65535); fix: .config/factory.toml"
        )),
    }
}

fn path(config: &Config) -> Result<String, String> {
    let text = required(config, "serve.path")?;
    if text.starts_with('/') && text.ends_with('/') {
        Ok(text.to_owned())
    } else {
        Err(format!(
            "serve.path = \"{text}\" must start and end with /, such as / or /vapors/; fix: .config/factory.toml"
        ))
    }
}

#[cfg(test)]
#[path = "serve_config_tests.rs"]
mod tests;
