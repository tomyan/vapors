//! Where `cargo xtask serve` and `cargo xtask dev` put a page on the
//! tailnet, from `.config/factory.toml`: `[serve]` for the release build
//! (`dir` under the home directory, `https`, `path`, and `check`, the
//! command that must pass on `target/web` first) and `[dev] https` for
//! trunk's debug page, which is always at `/` of its own port.

use std::path::{Path, PathBuf};

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
    /// A shell line run in the checkout; the build is served only when
    /// it exits 0.
    pub check: String,
}

/// `[serve]`, with `dir` under `home`.
///
/// # Errors
///
/// A key missing or malformed, naming it and the fix.
pub fn serve(config: &Config, home: &Path) -> Result<Serve, String> {
    let dir = required(config, "serve.dir")?;
    if dir.starts_with('/') || dir.contains("..") {
        return Err(format!(
            "[serve] dir = \"{dir}\" must be a path under the home directory, such as srv/<name>; fix: .config/factory.toml"
        ));
    }
    Ok(Serve {
        dir: home.join(dir),
        mount: Mount {
            https: port(config, "serve.https")?,
            path: path(config)?,
        },
        check: required(config, "serve.check")?.to_owned(),
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
