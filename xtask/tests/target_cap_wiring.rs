//! Every build xtask starts is refused while the worktree's build
//! directory is over `[targets] max_gb`: `xtask ci`, `xtask test` and
//! `xtask web` run as processes in a scratch workspace whose cap is
//! 0.0001 GB (100 kB) and whose target holds a 200 kB file in
//! `debug/deps`. The cap is checked before sccache is asked for, so no
//! cache or server is needed to see the refusal.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CONFIG: &str =
    "[web]\npackage = \"cap-web\"\n\n[load]\nwait = \"false\"\n\n[targets]\nmax_gb = \"0.0001\"\n";

const MANIFEST: &str =
    "[package]\nname = \"cap-web\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[workspace]\n";

const LOCK: &str = "version = 4\n\n[[package]]\nname = \"cap-web\"\nversion = \"0.0.0\"\n";

fn scratch(name: &str, deps_bytes: usize) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(dir.join(".config")).expect("scratch dir");
    std::fs::write(dir.join(".config/factory.toml"), CONFIG).expect("config");
    // a package for `xtask web` to find before it builds
    std::fs::create_dir_all(dir.join("src")).expect("src");
    std::fs::write(dir.join("src/lib.rs"), "").expect("lib");
    std::fs::write(dir.join("Cargo.toml"), MANIFEST).expect("manifest");
    std::fs::write(dir.join("Cargo.lock"), LOCK).expect("lock");
    std::fs::create_dir_all(dir.join("target/debug/deps")).expect("deps");
    std::fs::write(
        dir.join("target/debug/deps/big.rlib"),
        vec![7u8; deps_bytes],
    )
    .expect("big");
    dir
}

/// `xtask <args>` in `dir`, with HOME there so no real lock is waited on.
fn xtask(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .current_dir(dir)
        .env("HOME", dir)
        .env_remove("GITHUB_ACTIONS")
        .env_remove("XTASK_GATE")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("xtask runs")
}

fn said(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn ci_test_and_web_refuse_to_build_over_the_cap_naming_the_command() {
    for args in [&["ci"][..], &["test"], &["web"]] {
        // Given a target over its cap
        let dir = scratch(&format!("cap-over-{}", args[0]), 200_000);

        // When
        let output = xtask(&dir, args);

        // Then
        let text = said(&output);
        let real = dir.join("target").canonicalize().expect("real");
        let command = format!(
            "find '{}' -mindepth 1 -maxdepth 1 -exec rm -rf {{}} +",
            real.display()
        );
        assert!(!output.status.success(), "{args:?} passed: {text}");
        assert!(text.contains("over [targets] max_gb"), "{args:?}: {text}");
        assert!(text.contains(&command), "{args:?}: {text}");
    }
}

#[test]
fn a_run_from_a_subdirectory_of_the_workspace_is_capped_too() {
    // Given a target over its cap, and a run from inside src/
    let dir = scratch("cap-over-subdir", 200_000);

    // When
    let output = xtask(&dir.join("src"), &["test"]);

    // Then
    let text = said(&output);
    assert!(!output.status.success(), "passed: {text}");
    assert!(text.contains("over [targets] max_gb"), "{text}");
}
