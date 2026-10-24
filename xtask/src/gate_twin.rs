//! The gate's twin step. touche and vapors keep one xtask, and changes
//! land in vapors first, so only touche checks: `[gate] twin` names the
//! vapors checkout, which `[gate.pinned]` holds at touche's pin. vapors
//! sets no twin and says who checks it.

use std::path::Path;

use crate::twin::{differences, fix_line};

/// The config key naming the sibling checkout.
pub const KEY: &str = "gate.twin";

/// What to print when `here`'s xtask matches `twin`'s (or no twin is
/// configured), or why not.
pub fn check(here: &Path, twin: Option<&Path>) -> Result<String, String> {
    let Some(twin) = twin else {
        return Ok(
            "twin: not checked here; touche checks its xtask against vapors at its pin".to_owned(),
        );
    };
    let theirs = twin.join("xtask");
    if !theirs.is_dir() {
        return Err(format!(
            "[gate] twin is {}, but {} is absent; fix: check out vapors there at .config/vapors-rev",
            twin.display(),
            theirs.display()
        ));
    }
    let found = differences(&here.join("xtask"), &theirs)?;
    if found.is_empty() {
        return Ok(format!(
            "ok   twin     xtask is the same as {}",
            theirs.display()
        ));
    }
    Err(format!(
        "xtask differs from {}:\n     {}\n     {}",
        theirs.display(),
        found.join("\n     "),
        fix_line(here, twin)
    ))
}

#[cfg(test)]
#[path = "gate_twin_tests.rs"]
mod tests;
