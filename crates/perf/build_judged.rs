//! The build script's decisions, kept here so the crate's tests
//! (`build_profile_tests.rs`) run them: the profile a build is for, named
//! from `OUT_DIR`, and for the judged profile the overrides refused, any
//! that could change its one codegen unit. Read from what a build script
//! can see: cargo's environment (`CARGO_PROFILE_JUDGED_*`, the encoded
//! and plain rustflags) and the cargo config files on the path from the
//! package up and in cargo's home.
//!
//! Not seen, so not refused: `cargo --config profile.judged.codegen-units=16`
//! (cargo gives a build script no trace of it), and a config that sets
//! the value in an inline table (`profile = { judged = { codegen-units =
//! 16 } }` or `judged = { codegen-units = 16 }` under `[profile]`), which
//! this line reader does not parse. Reading the config with a TOML parser
//! would retire the second; the first needs cargo to expose the profile.

use std::path::{Path, PathBuf};

/// The profile directory cargo builds into (`release`, `judged`, `debug`),
/// from `out_dir`, which is `<target>[/<triple>]/<profile>/build/<pkg>/out`.
/// Cargo gives a build script no other name for a custom profile.
///
/// # Errors
/// A layout without `build` two levels up (cargo's unstable build-dir has
/// `<profile>/build/<pkg>/<hash>/out`), naming it.
pub fn profile_of(out_dir: &Path) -> Result<String, String> {
    let name = |n: usize| out_dir.ancestors().nth(n).and_then(Path::file_name);
    match (name(2), name(3)) {
        (Some(build), Some(profile)) if build == "build" => {
            Ok(profile.to_string_lossy().into_owned())
        }
        _ => Err(format!(
            "OUT_DIR {} is not <target>/<profile>/build/<pkg>/out, so the profile cannot be named; \
             fix: teach crates/perf/build_judged.rs this layout",
            out_dir.display()
        )),
    }
}

/// Whether a build for `profile` may go on: any profile but `judged`
/// always, and `judged` with no override.
///
/// # Errors
/// The judged profile overridden, naming each override and the fix.
pub fn refusal(
    profile: &str,
    env: &[(String, String)],
    configs: &[(PathBuf, String)],
) -> Result<(), String> {
    if profile != "judged" {
        return Ok(());
    }
    let found = overrides(env, configs);
    if found.is_empty() {
        return Ok(());
    }
    Err(format!(
        "the judged profile is release with one codegen unit, and these override it: {}; \
         fix: remove them for a judged build, or build --release for an unjudged one",
        found.join("; ")
    ))
}

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
