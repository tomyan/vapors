//! The overrides the build script refuses for the judged profile: any
//! that could change its one codegen unit. Read from what a build script
//! can see: cargo's environment (`CARGO_PROFILE_JUDGED_*`, the encoded
//! and plain rustflags) and the cargo config files on the path from the
//! package up and in cargo's home. Shared with the crate's tests
//! (`build_profile_tests.rs`).

use std::path::{Path, PathBuf};

/// Each override of the judged profile found, as a line naming where.
pub fn overrides(env: &[(String, String)], configs: &[(PathBuf, String)]) -> Vec<String> {
    let mut found: Vec<String> = env
        .iter()
        .filter(|(name, value)| profile_var(name) || rustflags_set_units(name, value))
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    for (path, text) in configs {
        found.extend(config_units(path, text));
    }
    found
}

fn profile_var(name: &str) -> bool {
    name.starts_with("CARGO_PROFILE_JUDGED_")
}

/// Whether rustflags `value` (`name` one of the rustflags variables,
/// encoded with 0x1f between flags or plain with spaces) set codegen units.
fn rustflags_set_units(name: &str, value: &str) -> bool {
    let flags = matches!(
        name,
        "CARGO_ENCODED_RUSTFLAGS" | "RUSTFLAGS" | "CARGO_BUILD_RUSTFLAGS"
    );
    flags && value.contains("codegen-units")
}

/// The lines of a cargo config that set the judged profile's codegen
/// units, in a `[profile.judged]` table or as a dotted key.
fn config_units(path: &Path, text: &str) -> Vec<String> {
    let mut table = String::new();
    let mut found = Vec::new();
    for (at, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.starts_with('[') {
            table = line.replace(' ', "");
            continue;
        }
        let dotted = line.starts_with("profile.judged.codegen-units");
        if dotted || (table == "[profile.judged]" && line.starts_with("codegen-units")) {
            found.push(format!("{}:{}: {line}", path.display(), at + 1));
        }
    }
    found
}

/// The cargo config files cargo reads for a package at `manifest_dir`:
/// `.cargo/config.toml` and `.cargo/config` in it and every directory
/// above, and `config.toml` and `config` in `cargo_home`.
pub fn config_paths(manifest_dir: &Path, cargo_home: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = manifest_dir
        .ancestors()
        .flat_map(|dir| [dir.join(".cargo/config.toml"), dir.join(".cargo/config")])
        .collect();
    paths.extend([cargo_home.join("config.toml"), cargo_home.join("config")]);
    paths
}
