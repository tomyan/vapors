//! The build directory's cap: its measure, its setting, its refusal.

use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use super::*;

/// `bytes` of data in a new file at `path`, and what the file system
/// allocated for it.
fn file(path: &Path, bytes: usize) -> u64 {
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("dirs");
    std::fs::write(path, vec![7u8; bytes]).expect("written");
    std::fs::metadata(path).expect("there").blocks() * 512
}

#[test]
fn the_measure_counts_deps_examples_and_incremental_under_each_profile() {
    // Given a host and a cross-compiled profile, and files the measure
    // leaves out
    let target = crate::scratch::dir("cap-measure");
    let counted = [
        file(&target.join("debug/deps/libvapors.rlib"), 300_000),
        file(&target.join("debug/examples/cards"), 250_000),
        file(
            &target.join("debug/incremental/app-1x/s-2y/query.bin"),
            150_000,
        ),
        file(
            &target.join("wasm32-unknown-unknown/release/deps/app.wasm"),
            200_000,
        ),
    ];
    file(&target.join("debug/build/out/big.bin"), 500_000);
    file(&target.join("debug/deps/nested/below.bin"), 400_000);
    file(&target.join("tmp/scratch.bin"), 100_000);

    // When
    let held = measure(&target).expect("measured");

    // Then
    assert_eq!(held, counted.iter().sum::<u64>());
}

#[test]
fn a_target_without_builds_holds_nothing() {
    let target = crate::scratch::dir("cap-empty");
    assert_eq!(measure(&target), Ok(0));
}

#[test]
fn a_directory_that_cannot_be_read_is_refused_naming_it_not_counted_as_zero() {
    // Given a deps directory no one may list
    let target = crate::scratch::dir("cap-unreadable");
    file(&target.join("debug/deps/libvapors.rlib"), 1000);
    let deps = target.join("debug/deps");
    std::fs::set_permissions(&deps, std::fs::Permissions::from_mode(0o000)).expect("chmod");

    // When
    let measured = measure(&target);
    std::fs::set_permissions(&deps, std::fs::Permissions::from_mode(0o755)).expect("chmod back");

    // Then
    let refusal = measured.expect_err("refused");
    assert!(refusal.contains(&deps.display().to_string()), "{refusal}");
}

#[test]
fn the_workspace_is_found_from_a_subdirectory() {
    // Given a workspace and a directory deep inside it
    let root = crate::scratch::dir("cap-root");
    std::fs::create_dir_all(root.join(".config")).expect("config dir");
    std::fs::write(root.join(".config/factory.toml"), "").expect("settings");
    let deep = root.join("crates/layout/src");
    std::fs::create_dir_all(&deep).expect("deep");

    // When / Then
    assert_eq!(workspace_root(&deep).as_deref(), Ok(&*root.to_path_buf()));
    let outside = crate::scratch::dir("cap-no-root");
    assert!(workspace_root(&outside).is_err());
}

#[test]
fn cargo_target_dir_names_the_directory_measured() {
    // Given a workspace whose own target/ is not the one cargo uses
    let root = crate::scratch::dir("cap-env-root");
    std::fs::create_dir_all(root.join("target")).expect("target");
    let elsewhere = crate::scratch::dir("cap-env-target");

    // When
    let found = target_dir(&root, Some(elsewhere.as_os_str()));

    // Then
    assert_eq!(found, Ok(Some(elsewhere.to_path_buf())));
    assert_eq!(target_dir(&root, None), Ok(Some(root.join("target"))));
}

#[test]
fn a_target_dir_in_cargo_config_is_refused_naming_the_line() {
    // Given
    let root = crate::scratch::dir("cap-config-dir");
    std::fs::create_dir_all(root.join(".cargo")).expect("cargo dir");
    std::fs::write(
        root.join(".cargo/config.toml"),
        "[build]\ntarget-dir = \"/x\"\n",
    )
    .expect("config");

    // When
    let refusal = target_dir(&root, None).expect_err("refused");

    // Then
    assert!(refusal.contains(".cargo/config.toml:2"), "{refusal}");
    assert!(refusal.contains("CARGO_TARGET_DIR"), "{refusal}");
}

#[test]
fn over_the_cap_is_refused_naming_the_size_how_to_see_it_and_the_command() {
    // Given
    let target = crate::scratch::dir("cap-over");

    // When: 80.6 GB held under a cap of 80
    let refusal = over(&target, 80_600_000_000, 80.0).expect_err("refused");

    // Then: one decimal, so the size is never the cap's; the path quoted
    let real = target.canonicalize().expect("real");
    let command = format!(
        "find '{}' -mindepth 1 -maxdepth 1 -exec rm -rf {{}} +",
        real.display()
    );
    assert!(refusal.contains("80.6 GB"), "{refusal}");
    assert!(refusal.contains("[targets] max_gb = 80"), "{refusal}");
    assert!(
        refusal.contains(&format!("du -sh '{}'", real.display())),
        "{refusal}"
    );
    assert!(refusal.contains(&command), "{refusal}");
}

#[test]
fn at_or_under_the_cap_the_build_goes_on() {
    let target = crate::scratch::dir("cap-under");
    assert_eq!(over(&target, 80 * GB, 80.0), Ok(()));
    assert_eq!(over(&target, 0, 80.0), Ok(()));
}

#[test]
fn the_cap_is_a_number_of_gb_above_0_or_unset() {
    assert_eq!(cap(Some("80")), Ok(Some(80.0)));
    assert_eq!(cap(Some("0.5")), Ok(Some(0.5)));
    assert_eq!(cap(None), Ok(None));
    for bad in ["0", "80GB", "-1", "inf", "NaN", ""] {
        let refusal = cap(Some(bad)).expect_err("refused");
        assert!(refusal.contains("[targets] max_gb"), "{bad}: {refusal}");
    }
}

#[test]
fn a_path_with_a_quote_is_quoted_for_the_shell() {
    assert_eq!(quoted(Path::new("/a b/c'd")), r"'/a b/c'\''d'");
}
