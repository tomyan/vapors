//! What a builder's branch may change before the harness pushes it
//! (`docs/plans/api-builder.md`, the lead's answer 8): only files under
//! the allowed paths (`crates/` and `docs/` unless the run names more),
//! no binary file, and at most [`LINE_CAP`] lines changed, the PR rule.

/// The paths a branch may change unless the run names more: `xtask/`,
/// `.config/`, `.github/`, hooks and `rust-toolchain.toml` change what
/// runs outside the sandbox, so each needs the run to say so.
pub const ALLOWED: [&str; 2] = ["crates/", "docs/"];

/// The most lines a pushed branch may change, added and removed: the
/// PR rule (about 400).
pub const LINE_CAP: u64 = 400;

/// `git diff --numstat` of the branch against its base, checked; the
/// lines changed.
///
/// # Errors
///
/// A file outside `allowed`, a binary file, or more than [`LINE_CAP`]
/// lines, each listed; or a numstat line of a shape this does not read.
pub fn checked(numstat: &str, allowed: &[String]) -> Result<u64, String> {
    let mut lines = 0;
    let mut refused = Vec::new();
    for row in numstat.lines().filter(|row| !row.trim().is_empty()) {
        let (added, removed, path) = fields(row)?;
        if !allowed
            .iter()
            .any(|prefix| path.starts_with(prefix.as_str()))
        {
            refused.push(format!("{path} is outside {}", allowed.join(", ")));
        }
        match (added.parse::<u64>(), removed.parse::<u64>()) {
            (Ok(added), Ok(removed)) => lines += added + removed,
            _ => refused.push(format!("{path} is binary")),
        }
    }
    if lines > LINE_CAP {
        refused.push(format!(
            "{lines} lines changed, over {LINE_CAP}: split the slice"
        ));
    }
    if refused.is_empty() {
        return Ok(lines);
    }
    Err(refused.join("; "))
}

fn fields(row: &str) -> Result<(&str, &str, &str), String> {
    let mut parts = row.splitn(3, '\t');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(added), Some(removed), Some(path)) => Ok((added, removed, path)),
        _ => Err(format!(
            "a `git diff --numstat` line this does not read: {row:?}"
        )),
    }
}

/// The allowed paths: [`ALLOWED`] and those the run names.
pub fn allowed(named: &[String]) -> Vec<String> {
    let mut all: Vec<String> = ALLOWED.map(str::to_owned).to_vec();
    all.extend(named.iter().cloned());
    all
}

#[cfg(test)]
#[path = "api_push_checks_tests.rs"]
mod tests;
