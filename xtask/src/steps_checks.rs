//! The lint tier's checks of the tree: files, layers, builds and
//! imports, and the advisory quality signals.

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

/// The quality signals, printed whether or not the step passes; it
/// fails only when it cannot measure. `cargo xtask quality`.
pub fn quality() -> Step {
    let mut step = Step::new(
        "quality",
        "cargo",
        &["xtask", "quality"],
        "make the file it names parse, or list it under [files.generated]; a missing threshold is set in clippy.toml or [quality]",
    );
    step.advisory = true;
    step
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
