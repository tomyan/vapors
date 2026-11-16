//! Compile the Mount workload's components, and record the revision the
//! harness is built from (`git describe --always --dirty`), which
//! `--scene-run` prints so a recorded run names the code it timed.
use std::process::Command;

#[path = "build_judged.rs"]
mod build_judged;

fn main() {
    vapors_compiler::build::compile_dir("ui");
    println!("cargo:rustc-env=VAPORS_PERF_REVISION={}", revision());
    let profile = profile_dir();
    if profile == "judged" {
        refuse_overrides();
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

/// The profile directory cargo builds this into (`release`, `judged`,
/// `debug`): `OUT_DIR` is `<target>[/<triple>]/<profile>/build/<pkg>/out`.
/// Cargo gives a build script no other name for a custom profile. A
/// layout without `build` two levels up (cargo's unstable build-dir has
/// `<profile>/build/<pkg>/<hash>/out`) fails the build naming that.
fn profile_dir() -> String {
    let out = std::env::var("OUT_DIR").expect("cargo sets OUT_DIR");
    let out = std::path::Path::new(&out);
    let name = |n: usize| {
        out.ancestors()
            .nth(n)
            .and_then(|dir| dir.file_name())
            .map(|name| name.to_string_lossy().into_owned())
    };
    assert!(
        name(2).as_deref() == Some("build"),
        "OUT_DIR {} is not <target>/<profile>/build/<pkg>/out, so the profile cannot be named; \
         fix: teach crates/perf/build.rs this layout",
        out.display()
    );
    name(3).expect("a profile directory above build/")
}

/// Fails the build when anything it can see overrides the judged
/// profile's one codegen unit (`build_judged.rs`).
fn refuse_overrides() {
    let env: Vec<(String, String)> = std::env::vars().collect();
    for (name, _) in env
        .iter()
        .filter(|(name, _)| name.starts_with("CARGO_PROFILE_JUDGED_"))
    {
        println!("cargo:rerun-if-env-changed={name}");
    }
    for name in [
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTFLAGS",
        "CARGO_BUILD_RUSTFLAGS",
        "CARGO_PROFILE_JUDGED_CODEGEN_UNITS",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    let manifest =
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let home = std::env::var_os("CARGO_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| std::path::Path::new(&home).join(".cargo")))
        .expect("CARGO_HOME or HOME");
    let configs: Vec<(std::path::PathBuf, String)> = build_judged::config_paths(&manifest, &home)
        .into_iter()
        .filter_map(|path| {
            println!("cargo:rerun-if-changed={}", path.display());
            std::fs::read_to_string(&path).ok().map(|text| (path, text))
        })
        .collect();
    let found = build_judged::overrides(&env, &configs);
    assert!(
        found.is_empty(),
        "the judged profile is release with one codegen unit, and these override it: {}; \
         fix: remove them for a judged build, or build --release for an unjudged one",
        found.join("; ")
    );
}
