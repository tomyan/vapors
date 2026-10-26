//! Tools fetched as a release archive from a pinned URL, not built or
//! installed by binstall: `[downloads]` in `.config/tools.toml`, one
//! `<name>.<field> = "<value>"` line per field. The archive's SHA-256 and
//! the binary's are both pinned; the binary is installed to a fixed path
//! under the home directory (`installed_path`) only after both match, and
//! is never run to find out what it is (`download_install.rs`).

use std::path::{Path, PathBuf};

use crate::tools::ParseError;

/// One pinned download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Download {
    pub name: String,
    pub version: String,
    /// An https URL of a `.tar.gz` release asset.
    pub url: String,
    /// The archive's SHA-256, lower-case hex.
    pub sha256: String,
    /// The binary's path inside the archive.
    pub member: String,
    /// The binary's SHA-256, checked after extracting and before any use.
    pub binary_sha256: String,
}

/// The tools file `[downloads]` is read from.
const TOOLS_FILE: &str = ".config/tools.toml";

/// The download pinned as `name` in the tools file.
///
/// # Errors
///
/// The file unread, malformed, or without `name`.
pub fn pinned(name: &str) -> Result<Download, String> {
    let text = std::fs::read_to_string(TOOLS_FILE)
        .map_err(|e| format!("could not read {TOOLS_FILE}: {e}; run from the workspace root"))?;
    parse(&text)
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|d| d.name == name)
        .ok_or_else(|| format!("{TOOLS_FILE} pins no {name} under [downloads]; fix: add it"))
}

const FIELDS: [&str; 5] = ["version", "url", "sha256", "member", "binary_sha256"];

/// Where `download` is installed: `~/.local/share/factory/bin/<name>-<version>`.
#[must_use]
pub fn installed_path(download: &Download, home: &Path) -> PathBuf {
    home.join(".local/share/factory/bin")
        .join(format!("{}-{}", download.name, download.version))
}

/// The `[downloads]` table of `text`, every field present and well formed.
///
/// # Errors
///
/// A field unknown, missing, repeated or malformed, naming the line.
pub fn parse(text: &str) -> Result<Vec<Download>, ParseError> {
    let mut fields: Vec<(String, String, String, usize)> = Vec::new();
    let mut inside = false;
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.starts_with('[') {
            inside = line == "[downloads]";
        } else if inside && !line.is_empty() && !line.starts_with('#') {
            fields.push(field(line, index + 1)?);
        }
    }
    collect(&fields)
}

fn field(line: &str, number: usize) -> Result<(String, String, String, usize), ParseError> {
    let refuse = |reason: String| ParseError {
        line: number,
        reason,
    };
    let (key, value) = line
        .split_once('=')
        .ok_or_else(|| refuse(format!("expected `<name>.<field> = \"value\"`: `{line}`")))?;
    let value = value
        .trim()
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'));
    let value = value.filter(|v| !v.is_empty()).ok_or_else(|| {
        refuse(format!(
            "value must be a non-empty double-quoted string: `{line}`"
        ))
    })?;
    let (name, kind) = key
        .trim()
        .rsplit_once('.')
        .ok_or_else(|| refuse(format!("key must be `<name>.<field>`: `{line}`")))?;
    if !FIELDS.contains(&kind) {
        return Err(refuse(format!(
            "unknown field `{kind}`; one of {}",
            FIELDS.join(", ")
        )));
    }
    check_value(kind, value).map_err(refuse)?;
    Ok((name.to_owned(), kind.to_owned(), value.to_owned(), number))
}

fn check_value(kind: &str, value: &str) -> Result<(), String> {
    let hex = value.len() == 64 && value.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'));
    match kind {
        "sha256" | "binary_sha256" if !hex => Err(format!(
            "{kind} must be 64 lower-case hex digits (a SHA-256)"
        )),
        "url" if !value.starts_with("https://") || !value.ends_with(".tar.gz") => {
            Err("url must be an https URL of a .tar.gz release asset".to_owned())
        }
        _ => Ok(()),
    }
}

fn collect(fields: &[(String, String, String, usize)]) -> Result<Vec<Download>, ParseError> {
    let mut names: Vec<String> = Vec::new();
    for (name, ..) in fields {
        if !names.contains(name) {
            names.push(name.clone());
        }
    }
    names.iter().map(|name| one(name, fields)).collect()
}

fn one(name: &str, fields: &[(String, String, String, usize)]) -> Result<Download, ParseError> {
    let get = |kind: &str| -> Result<String, ParseError> {
        let found: Vec<_> = fields
            .iter()
            .filter(|f| f.0 == name && f.1 == kind)
            .collect();
        let line = fields.iter().find(|f| f.0 == name).map_or(0, |f| f.3);
        match found.as_slice() {
            [only] => Ok(only.2.clone()),
            [] => Err(ParseError {
                line,
                reason: format!("[downloads] {name} has no {kind}"),
            }),
            [_, again, ..] => Err(ParseError {
                line: again.3,
                reason: format!("[downloads] {name}.{kind} is given twice"),
            }),
        }
    };
    Ok(Download {
        name: name.to_owned(),
        version: get("version")?,
        url: get("url")?,
        sha256: get("sha256")?,
        member: get("member")?,
        binary_sha256: get("binary_sha256")?,
    })
}

#[cfg(test)]
#[path = "downloads_tests.rs"]
mod tests;
