//! What must work inside the sandbox, checked with the fences
//! (`api_fences_run.rs`): each a script run inside and what its output
//! must be. The way in and the toolchain show the sandbox is usable; the
//! rest pin fences an escape probe cannot show from outside (its own
//! namespaces, its own session, only the bound home entries, only the
//! clone's two refs, no inherited file descriptor), each proven by a
//! break in the sandbox's arguments.

use crate::api_fences::{Ran, SENTINEL};

/// One thing that must work inside: the script, and what its output
/// (the sentinel's line aside) must satisfy, described.
pub struct Works {
    pub name: &'static str,
    pub script: String,
    pub expected: Expected,
}

/// What a check's output must be.
pub enum Expected {
    /// Exactly this.
    Is(String),
    /// None of these lines: the host's own values.
    NoneOf(Vec<String>),
}

/// Each check run by `inside`; a line for each.
///
/// # Errors
///
/// Every check that failed, with what it said.
pub fn checked(
    works: &[Works],
    inside: &dyn Fn(&str) -> Result<Ran, String>,
) -> Result<Vec<String>, String> {
    let mut lines = Vec::new();
    let mut failed = Vec::new();
    for check in works {
        let ran = inside(&check.script)?;
        let output = without_sentinel(&ran.said);
        if ran.success && check.expected.met(&output) {
            lines.push(format!("works inside: {}", check.name));
        } else {
            failed.push(format!(
                "{}: `{}` said {output:?}",
                check.name, check.script
            ));
        }
    }
    if failed.is_empty() {
        return Ok(lines);
    }
    Err(format!(
        "inside the sandbox, {}; the sandbox's arguments are api_sandbox.rs, reproduce with \
         `cargo xtask api-fences <name>`",
        failed.join("; ")
    ))
}

impl Expected {
    fn met(&self, output: &str) -> bool {
        match self {
            Self::Is(expected) => output == expected,
            Self::NoneOf(host) => !output
                .lines()
                .any(|line| host.iter().any(|value| value == line)),
        }
    }
}

fn without_sentinel(said: &str) -> String {
    let lines: Vec<&str> = said.lines().filter(|line| *line != SENTINEL).collect();
    lines.join("\n")
}

/// The namespaces the sandbox must not share with the host, as the
/// host's own links name them (`ipc:[4026531839]`).
///
/// # Errors
///
/// A namespace link that cannot be read.
pub fn host_namespaces() -> Result<Vec<String>, String> {
    NAMESPACES
        .iter()
        .map(|name| {
            let link = format!("/proc/self/ns/{name}");
            std::fs::read_link(&link)
                .map(|target| target.display().to_string())
                .map_err(|e| format!("reading {link}: {e}"))
        })
        .collect()
}

/// The namespaces bwrap unshares (`api_sandbox.rs` `isolation`).
pub const NAMESPACES: [&str; 5] = ["ipc", "net", "pid", "uts", "cgroup"];

#[cfg(test)]
#[path = "api_fences_works_tests.rs"]
mod tests;
