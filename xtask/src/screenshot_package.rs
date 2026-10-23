//! Which package `cargo xtask screenshot` runs a target from: the one
//! package in `[screenshot] packages` that has an example (or, with
//! `--bin`, a binary) of that name, as `cargo metadata` lists them.

use std::process::Command;

use serde_json::Value;

use crate::config::Config;
use crate::screenshot::Target;

const KEY: &str = "screenshot.packages";

/// The `[screenshot] packages` list, comma-separated, each entry trimmed.
pub fn packages(config: &Config) -> Result<Vec<String>, String> {
    if config.get("screenshot.package").is_some() {
        return Err(
            "`[screenshot] package` is now a list: rename `package` to `packages` \
             (comma-separated, such as `packages = \"a, b\"`)"
                .to_owned(),
        );
    }
    let text = config.require(KEY).map_err(|e| e.to_string())?;
    let names: Vec<String> = text.split(',').map(|n| n.trim().to_owned()).collect();
    if names.iter().any(String::is_empty) {
        return Err(format!(
            "`[screenshot] packages = \"{text}\"` has an empty entry; \
             fix: list package names separated by single commas"
        ));
    }
    Ok(names)
}

/// The workspace's `cargo metadata --no-deps` as JSON text.
pub fn workspace_metadata() -> Result<String, String> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .map_err(|e| format!("could not run cargo metadata: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata --no-deps failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| format!("cargo metadata printed non-UTF-8: {e}"))
}

/// The one listed package that has `target`, from `metadata` (the JSON
/// text of `cargo metadata --no-deps`).
pub fn choose(listed: &[String], metadata: &str, target: &Target) -> Result<String, String> {
    let workspace = targets_by_package(metadata, kind_of(target))?;
    let mut having = Vec::new();
    for name in listed {
        let Some((_, targets)) = workspace.iter().find(|(package, _)| package == name) else {
            return Err(format!(
                "`{name}` is not a workspace package; fix: correct `[screenshot] packages` \
                 in .config/factory.toml"
            ));
        };
        if targets.contains(&target.name) {
            having.push(name.clone());
        }
    }
    one_of(&having, listed, &workspace, target)
}

fn one_of(
    having: &[String],
    listed: &[String],
    workspace: &[(String, Vec<String>)],
    target: &Target,
) -> Result<String, String> {
    let kind = kind_of(target);
    match having {
        [one] => Ok(one.clone()),
        [] => Err(format!(
            "no {kind} `{}` in the packages `[screenshot] packages` lists; they have: {}; \
             fix: name one of those, or add the package to the list",
            target.name,
            inventory(listed, workspace)
        )),
        _ => Err(format!(
            "{kind} `{}` is in more than one listed package ({}); \
             fix: rename one, or drop a package from `[screenshot] packages`",
            target.name,
            having.join(", ")
        )),
    }
}

fn inventory(listed: &[String], workspace: &[(String, Vec<String>)]) -> String {
    workspace
        .iter()
        .filter(|(package, _)| listed.contains(package))
        .map(|(package, targets)| format!("{package}: {}", targets.join(", ")))
        .collect::<Vec<_>>()
        .join("; ")
}

fn kind_of(target: &Target) -> &'static str {
    if target.is_binary { "bin" } else { "example" }
}

/// Each package's name with its targets of `kind`, in metadata order.
fn targets_by_package(metadata: &str, kind: &str) -> Result<Vec<(String, Vec<String>)>, String> {
    let refuse = |what: &str| format!("cargo metadata's answer is not the expected shape: {what}");
    let json: Value = serde_json::from_str(metadata).map_err(|e| refuse(&e.to_string()))?;
    let packages = json["packages"]
        .as_array()
        .ok_or_else(|| refuse("no `packages` array"))?;
    packages
        .iter()
        .map(|package| {
            let name = package["name"]
                .as_str()
                .ok_or_else(|| refuse("a package with no name"))?;
            Ok((
                name.to_owned(),
                targets_of(package, kind).ok_or_else(|| refuse(name))?,
            ))
        })
        .collect()
}

fn targets_of(package: &Value, kind: &str) -> Option<Vec<String>> {
    let mut names = Vec::new();
    for target in package["targets"].as_array()? {
        let kinds = target["kind"].as_array()?;
        if kinds.iter().any(|k| k.as_str() == Some(kind)) {
            names.push(target["name"].as_str()?.to_owned());
        }
    }
    Some(names)
}

#[cfg(test)]
#[path = "screenshot_package_tests.rs"]
mod tests;
