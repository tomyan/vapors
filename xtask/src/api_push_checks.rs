//! What a builder's branch may change before the harness pushes it
//! (`docs/plans/api-builder.md`, the lead's answer 8): only files under
//! the allowed paths (`crates/` and `docs/` unless the run names more),
//! no binary file, no symlink or submodule, and at most [`LINE_CAP`]
//! lines changed, the PR rule. The diff is read with `-z`, so a path is
//! its bytes, never git's quoting (review of slice 4, L1).

/// The paths a branch may change unless the run names more: `xtask/`,
/// `.config/`, `.github/`, hooks and `rust-toolchain.toml` change what
/// runs outside the sandbox, so each needs the run to say so.
pub const ALLOWED: [&str; 2] = ["crates/", "docs/"];

/// The most lines a pushed branch may change, added and removed: the
/// PR rule, raised from 400 by Tom on 2026-11-19: the last six harness
/// slices were 490 to 730 lines, most of it tests and docs.
pub const LINE_CAP: u64 = 600;

/// File modes refused: a symlink can point a checkout outside the tree,
/// and a gitlink names another repository.
const MODES: [(&str, &str); 2] = [("120000", "a symlink"), ("160000", "a submodule")];

/// The branch's diff against its base, checked: `numstat` is
/// `git diff --numstat -z`, `raw` is `git diff --raw -z`, both
/// `--no-renames`. The lines changed.
///
/// # Errors
///
/// Every refusal, listed: a path outside `allowed` or with a `..`, `.`
/// or `.git` part, a binary file, a symlink or submodule, more than
/// [`LINE_CAP`] lines; or output of a shape this does not read.
pub fn checked(numstat: &str, raw: &str, allowed: &[String]) -> Result<u64, String> {
    let mut refused = modes(raw)?;
    let mut lines = 0;
    for row in numstat.split('\0').filter(|row| !row.is_empty()) {
        let (added, removed, path) = fields(row)?;
        refused.extend(path_refused(path, allowed));
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
            "a `git diff --numstat -z` record this does not read: {row:?}"
        )),
    }
}

/// Why `path` may not change, if it may not.
fn path_refused(path: &str, allowed: &[String]) -> Option<String> {
    if path.contains('\u{fffd}') {
        return Some(format!("{path:?} is not UTF-8"));
    }
    if path
        .split('/')
        .any(|part| matches!(part, "" | "." | ".." | ".git"))
    {
        return Some(format!("{path:?} has an empty, `.`, `..` or `.git` part"));
    }
    if !allowed
        .iter()
        .any(|prefix| path.starts_with(prefix.as_str()))
    {
        return Some(format!("{path} is outside {}", allowed.join(", ")));
    }
    None
}

/// `git diff --raw -z`'s records (`:<old> <new> <ids> <status>\0<path>\0`):
/// one refusal for each symlink or submodule, before or after.
fn modes(raw: &str) -> Result<Vec<String>, String> {
    let mut records = raw.split('\0').filter(|part| !part.is_empty());
    let mut refused = Vec::new();
    while let Some(header) = records.next() {
        let path = records
            .next()
            .ok_or_else(|| format!("a `git diff --raw -z` record with no path: {header:?}"))?;
        let mut fields = header.trim_start_matches(':').split(' ');
        let (old, new) = (
            fields.next().unwrap_or_default(),
            fields.next().unwrap_or_default(),
        );
        if let Some((_, what)) = MODES.iter().find(|(mode, _)| *mode == old || *mode == new) {
            refused.push(format!("{path} is {what}"));
        }
    }
    Ok(refused)
}

/// The allowed paths: [`ALLOWED`] and those the run names, each a
/// relative directory prefix ending in `/`.
///
/// # Errors
///
/// A named path that is empty, absolute, has a `..` part, or does not end
/// in `/` (so `xtask` cannot also allow `xtask-other/`).
pub fn allowed(named: &[String]) -> Result<Vec<String>, String> {
    let bad = |path: &String| {
        path.starts_with('/')
            || !path.ends_with('/')
            || path.len() < 2
            || path.split('/').any(|part| part == "..")
    };
    if let Some(path) = named.iter().find(|path| bad(path)) {
        return Err(format!(
            "--allow {path:?}: name a relative directory ending in `/`, such as `xtask/`"
        ));
    }
    Ok(ALLOWED
        .iter()
        .map(|&path| path.to_owned())
        .chain(named.iter().cloned())
        .collect())
}

#[cfg(test)]
#[path = "api_push_checks_tests.rs"]
mod tests;
