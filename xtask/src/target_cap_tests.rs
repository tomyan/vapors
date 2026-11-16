//! The build directory's cap: its measure, its setting, its refusal.

use std::os::unix::fs::MetadataExt;
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
fn the_measure_sums_the_files_in_each_deps_directory_and_nothing_else() {
    // Given a target with a host profile, a cross-compiled profile, and
    // files outside deps and below it
    let target = crate::scratch::dir("cap-measure");
    let host = file(&target.join("debug/deps/libvapors.rlib"), 300_000);
    let wasm = file(
        &target.join("wasm32-unknown-unknown/release/deps/app.wasm"),
        200_000,
    );
    file(&target.join("debug/build/out/big.bin"), 500_000);
    file(&target.join("debug/deps/nested/below.bin"), 400_000);
    file(&target.join("tmp/scratch.bin"), 100_000);

    // When
    let held = deps_bytes(&target).expect("measured");

    // Then: the two deps listings' files only, as allocated
    assert_eq!(held, host + wasm);
}

#[test]
fn a_target_without_deps_holds_nothing() {
    let target = crate::scratch::dir("cap-empty");
    assert_eq!(deps_bytes(&target), Ok(0));
}

#[test]
fn over_the_cap_is_refused_naming_the_size_the_setting_and_the_command() {
    // Given
    let target = crate::scratch::dir("cap-over");

    // When: 81 GB held under a cap of 80
    let refusal = over(&target, 81 * GB, 80.0).expect_err("refused");

    // Then
    let real = target.canonicalize().expect("real");
    let command = format!(
        "find {} -mindepth 1 -maxdepth 1 -exec rm -rf {{}} +",
        real.display()
    );
    assert!(refusal.contains("81 GB"), "{refusal}");
    assert!(refusal.contains("[targets] max_gb = 80"), "{refusal}");
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
