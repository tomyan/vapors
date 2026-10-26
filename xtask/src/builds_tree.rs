//! Reading `cargo tree -e normal --prefix depth --format {p}`: which
//! server-only crates a shipped build's tree holds, each with the path
//! from the build's package that pulled it in.

/// Each of `server_only` in `tree`, with its first path from the root.
///
/// # Errors
///
/// A line in no shape this reads, quoted with its number.
pub fn pulled_in(tree: &str, server_only: &[String]) -> Result<Vec<String>, String> {
    let mut path: Vec<&str> = Vec::new();
    let mut found: Vec<(&str, String)> = Vec::new();
    for (index, line) in tree.lines().enumerate() {
        let said = |why: &str| format!("cargo tree's line {} {why}: {line:?}", index + 1);
        let (depth, name) =
            depth_and_name(line).ok_or_else(|| said("is not `<depth><name> v<version>`"))?;
        if depth > path.len() {
            return Err(said("is deeper than the line before it allows"));
        }
        path.truncate(depth);
        path.push(name);
        let listed = server_only.iter().any(|crate_name| crate_name == name);
        if listed && !found.iter().any(|(seen, _)| *seen == name) {
            found.push((name, path.join(" -> ")));
        }
    }
    Ok(found
        .into_iter()
        .map(|(name, path)| format!("{name}, pulled in by {path}"))
        .collect())
}

/// A line's depth and crate name: `3hyper v1.0.0 (*)` is 3 and `hyper`.
fn depth_and_name(line: &str) -> Option<(usize, &str)> {
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    let depth = line[..digits].parse().ok()?;
    let (name, version) = line[digits..].split_once(' ')?;
    version.starts_with('v').then_some((depth, name))
}

#[cfg(test)]
#[path = "builds_tree_tests.rs"]
mod tests;
