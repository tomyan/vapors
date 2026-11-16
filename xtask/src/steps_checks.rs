//! The lint tier's checks of the tree: files, layers, builds and imports,
//! and the nightly scaling runner's scripts.

use crate::step::Step;

/// No crate depends on what its `[layers.forbid]` rule names;
/// `cargo xtask check-layers`.
pub fn layers() -> Step {
    Step::new(
        "layers",
        "cargo",
        &["xtask", "check-layers"],
        "remove the dependency the line names, or move the code; the rules are [layers.forbid] in .config/factory.toml",
    )
}

/// No shipped build's tree holds a `[server_only]` crate;
/// `cargo xtask check-builds`.
pub fn builds() -> Step {
    Step::new(
        "builds",
        "cargo",
        &["xtask", "check-builds"],
        "remove the dependency the line names, or move the code out of what the build depends on; the lists are [server_only] in .config/factory.toml",
    )
}

/// A crate named in `[imports.<crate>]` only where listed;
/// `cargo xtask check-imports`.
pub fn imports() -> Step {
    Step::new(
        "imports",
        "cargo",
        &["xtask", "check-imports"],
        "move the call into an allowed module, or list the path under [imports.<crate>] in .config/factory.toml with a reason",
    )
}

/// Every source file within `[files] max_lines`; `cargo xtask check-files`.
pub fn files() -> Step {
    Step::new(
        "files",
        "cargo",
        &["xtask", "check-files"],
        "split the file, or allow it in .config/factory.toml under [files.allow] with a reason",
    )
}

/// The nightly scaling runner's guards: deadlines, lock, quoted error.
pub fn nightly() -> Step {
    nightly_script("nightly", "scripts/nightly-scaling-check.sh")
}

/// The nightly scaling runner's issue logic and run markers against
/// scripts/fixtures/nightly-scaling.log.
pub fn nightly_issues() -> Step {
    nightly_script("nightly issues", "scripts/nightly-scaling-issues-check.sh")
}

/// A check script with a fake git and gh: no network, no build. They
/// took 5 s and 2 s here (2026-11-13); a minute is over ten times that.
fn nightly_script(name: &str, script: &str) -> Step {
    let mut step = Step::new(
        name,
        script,
        &[],
        "each FAIL line names the guard and what happened; fix scripts/nightly-scaling.sh or scripts/nightly-scaling-issues.sh",
    );
    step.deadline = Some(std::time::Duration::from_mins(1));
    step
}
