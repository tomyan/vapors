//! Compile the Mount workload's components, and record the revision the
//! harness is built from (`git describe --always --dirty`), which
//! `--scene-run` prints so a recorded run names the code it timed.
use std::process::Command;

#[path = "build_judged.rs"]
mod build_judged;

fn main() {
    vapors_compiler::build::compile_dir("ui");
    println!("cargo:rustc-env=VAPORS_PERF_REVISION={}", revision());
    let out = std::env::var("OUT_DIR").expect("cargo sets OUT_DIR");
    let profile =
        build_judged::profile_of(std::path::Path::new(&out)).unwrap_or_else(|why| panic!("{why}"));
    let (env, configs) = what_overrides_read();
    if let Err(why) = build_judged::refusal(&profile, &env, &configs) {
        panic!("{why}");
    }
    println!("cargo:rustc-env=VAPORS_PERF_PROFILE_DIR={profile}");
    // the revision moves with every commit or checkout (HEAD's log), a
    // staged change (the index) and any edit to the tree
    for watched in [
        git(&["rev-parse", "--git-path", "logs/HEAD"]),
        git(&["rev-parse", "--git-path", "index"]),
    ]
    .into_iter()
    .flatten()
    .chain(["../../crates".into(), "../../Cargo.lock".into()])
    {
        println!("cargo:rerun-if-changed={watched}");
    }
}

/// `git args`'s output, or why it failed; git's own environment is
/// stripped, since a hook running cargo exports it.
fn git(args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .map_err(|error| format!("git: {error}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// The tree's revision, or `unknown (<why>)`.
fn revision() -> String {
    git(&["describe", "--always", "--dirty", "--abbrev=12"])
        .unwrap_or_else(|why| format!("unknown ({why})"))
}

/// A cargo config file's path and text.
type Config = (std::path::PathBuf, String);

/// The environment and the cargo config files that exist, which the
/// judged profile's overrides are read from (`build_judged.rs`), each
/// watched so a change reruns this script. A config file that does not
/// exist is not watched: cargo reruns a build script on every build while
/// a watched path is missing. Creating one is seen at the next run of
/// this script.
fn what_overrides_read() -> (Vec<(String, String)>, Vec<Config>) {
    let env: Vec<(String, String)> = std::env::vars().collect();
    let profile_vars = env
        .iter()
        .map(|(name, _)| name.as_str())
        .filter(|name| name.starts_with("CARGO_PROFILE_JUDGED_"));
    for name in [
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTFLAGS",
        "CARGO_BUILD_RUSTFLAGS",
        "CARGO_PROFILE_JUDGED_CODEGEN_UNITS",
    ]
    .into_iter()
    .chain(profile_vars)
    {
        println!("cargo:rerun-if-env-changed={name}");
    }
    let manifest =
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let home = std::env::var_os("CARGO_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| std::path::Path::new(&home).join(".cargo")))
        .expect("CARGO_HOME or HOME");
    let configs = build_judged::config_paths(&manifest, &home)
        .into_iter()
        .filter_map(read_config)
        .collect();
    (env, configs)
}

/// The config file at `path` and its text, watched; `None` when it does
/// not exist; a file that exists but cannot be read fails the build.
fn read_config(path: std::path::PathBuf) -> Option<Config> {
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            println!("cargo:rerun-if-changed={}", path.display());
            Some((path, text))
        }
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => None,
        Err(why) => panic!(
            "cannot read cargo config {}: {why}; fix: make it readable",
            path.display()
        ),
    }
}
