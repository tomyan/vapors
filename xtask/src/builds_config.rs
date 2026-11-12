//! `[server_only]` in .config/factory.toml: the crates only the hosted or
//! corporate product may use, and each shipped build to keep them out of.

use crate::config::Config;

/// One shipped build: its name, package and target (none for the host).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Build {
    pub name: String,
    pub package: String,
    pub target: Option<String>,
}

/// The server-only crates and the shipped builds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Separation {
    pub crates: Vec<String>,
    pub builds: Vec<Build>,
}

impl Separation {
    /// The settings in `config`.
    ///
    /// # Errors
    ///
    /// A setting missing or in another shape, named with its fix.
    pub fn from(config: &Config) -> Result<Self, String> {
        let crates: Vec<String> = config
            .get("server_only.crates")
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        reason_holds(&crates, config.get("server_only.none_because"))?;
        Ok(Self {
            crates,
            builds: builds(config)?,
        })
    }
}

/// No crates says why (the config refuses an empty value, so the key is
/// left out), and a list does not.
fn reason_holds(crates: &[String], reason: Option<&str>) -> Result<(), String> {
    let reason = reason.is_some_and(|text| !text.trim().is_empty());
    match (crates.is_empty(), reason) {
        (true, false) => Err("[server_only] lists no crates and none_because gives no reason; fix: say why nothing is server-only yet, or list the crates".into()),
        (false, true) => Err("[server_only] lists crates, so none_because no longer holds; fix: remove none_because".into()),
        _ => Ok(()),
    }
}

fn builds(config: &Config) -> Result<Vec<Build>, String> {
    let entries = config.entries_under("server_only.builds.");
    if entries.is_empty() {
        return Err("[server_only.builds] names no shipped build; fix: list each as <name> = \"<package> <target>\" (target host for the machine's own)".into());
    }
    entries
        .iter()
        .map(|(name, value)| build(name, value))
        .collect()
}

/// `<name> = "<package> <target>"`, `host` for no `--target`.
fn build(name: &str, value: &str) -> Result<Build, String> {
    let words: Vec<&str> = value.split_whitespace().collect();
    let [package, target] = words[..] else {
        return Err(format!(
            "[server_only.builds] {name} is `{value}`, not `<package> <target>`; fix: write the package and its target triple, or host"
        ));
    };
    Ok(Build {
        name: name.to_owned(),
        package: package.to_owned(),
        target: (target != "host").then(|| target.to_owned()),
    })
}

#[cfg(test)]
#[path = "builds_config_tests.rs"]
mod tests;
