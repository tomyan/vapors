//! Which build `cargo xtask web` asks trunk for, and where it goes.

use super::*;

#[test]
fn the_page_is_a_release_build_into_target_web() {
    // When
    let build = Build::of(false);

    // Then
    assert_eq!(
        build.trunk_args(),
        ["build", "--release", "--public-url", "./", "--dist"]
    );
    assert_eq!(build.dist, "target/web");
}

#[test]
fn the_debug_page_is_a_debug_build_into_its_own_directory() {
    // When
    let build = Build::of(true);

    // Then
    assert_eq!(
        build.trunk_args(),
        ["build", "--public-url", "./", "--dist"]
    );
    assert_eq!(build.dist, "target/web-debug");
}

#[test]
fn both_builds_use_a_relative_public_url_so_a_subpath_serves_them() {
    for debug in [false, true] {
        // When
        let args = Build::of(debug).trunk_args();

        // Then
        let at = args
            .iter()
            .position(|a| *a == "--public-url")
            .expect("a public URL");
        assert_eq!(args[at + 1], "./", "debug: {debug}");
        assert_eq!(args.last(), Some(&"--dist"), "debug: {debug}");
    }
}
