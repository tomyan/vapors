//! Where a crate may be named. A rule is `[imports.<crate>]`: each key is
//! a path that may name the crate, with its reason. A key ending in `/`
//! allows every file under that directory; any other key allows that one
//! file.

use std::collections::BTreeMap;

/// The allowed paths, with reasons, for each crate name.
pub type Rules = BTreeMap<String, BTreeMap<String, String>>;

/// The first line (from 1) that names `name` as a whole word in code;
/// a line whose code starts with `//` is a comment and does not count.
pub fn first_use(text: &str, name: &str) -> Option<usize> {
    text.lines()
        .position(|line| !line.trim_start().starts_with("//") && names(line, name))
        .map(|index| index + 1)
}

fn names(line: &str, name: &str) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    line.match_indices(name).any(|(at, _)| {
        let before = line[..at].chars().next_back();
        let after = line[at + name.len()..].chars().next();
        !before.is_some_and(word) && !after.is_some_and(word)
    })
}

/// Whether `path` is one an `allow` entry covers.
pub fn allowed(path: &str, allow: &BTreeMap<String, String>) -> bool {
    allow.keys().any(|entry| match entry.strip_suffix('/') {
        Some(dir) => path.starts_with(&format!("{dir}/")),
        None => path == entry,
    })
}

/// One line per file that names a crate outside its allowed paths.
pub fn violations(files: &[(String, String)], rules: &Rules) -> Vec<String> {
    let mut found = Vec::new();
    for (name, allow) in rules {
        let outside = files.iter().filter(|(path, _)| !allowed(path, allow));
        for (path, text) in outside {
            if let Some(line) = first_use(text, name) {
                found.push(finding(path, line, name, allow));
            }
        }
    }
    found
}

fn finding(path: &str, line: usize, name: &str, allow: &BTreeMap<String, String>) -> String {
    let paths: Vec<&str> = allow.keys().map(String::as_str).collect();
    format!(
        "{path}:{line} names {name}, which [imports.{name}] allows only in {}",
        paths.join(", ")
    )
}

#[cfg(test)]
#[path = "imports_rules_tests.rs"]
mod tests;
