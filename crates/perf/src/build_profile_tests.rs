use super::{JUDGED, judged};

#[test]
fn a_judged_run_built_with_another_profile_is_refused_naming_the_build() {
    for other in ["release", "debug", "bench"] {
        let refusal = judged(other).expect_err("refused");
        assert!(refusal.contains(other), "{refusal}");
        assert!(
            refusal.contains("cargo build --profile judged -p vapors-perf"),
            "{refusal}"
        );
    }
    assert_eq!(judged(JUDGED), Ok(()));
}

#[test]
fn the_judged_profile_is_release_with_one_codegen_unit() {
    // Given the workspace manifest's [profile.judged] table
    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml"))
            .expect("the workspace manifest");
    let table: Vec<&str> = manifest
        .split("[profile.judged]")
        .nth(1)
        .expect("a [profile.judged] table")
        .lines()
        .take_while(|line| !line.starts_with('['))
        .map(str::trim)
        .collect();

    // Then
    assert!(table.contains(&"inherits = \"release\""), "{table:?}");
    assert!(table.contains(&"codegen-units = 1"), "{table:?}");
}

#[path = "../build_judged.rs"]
mod build_judged;

#[test]
fn every_override_of_the_judged_profile_a_build_can_see_is_found() {
    // Given each kind of override, and settings that are not overrides
    let env = |pairs: &[(&str, &str)]| -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
            .collect()
    };
    let config = |text: &str| {
        vec![(
            std::path::PathBuf::from("/h/.cargo/config.toml"),
            text.to_owned(),
        )]
    };

    // Then
    let found = |e: &[(&str, &str)], c: &str| build_judged::overrides(&env(e), &config(c));
    assert_eq!(
        found(&[("CARGO_PROFILE_JUDGED_CODEGEN_UNITS", "16")], ""),
        ["CARGO_PROFILE_JUDGED_CODEGEN_UNITS=16"]
    );
    assert_eq!(
        found(&[("CARGO_ENCODED_RUSTFLAGS", "-C\x1fcodegen-units=16")], "").len(),
        1
    );
    assert_eq!(found(&[("RUSTFLAGS", "-C codegen-units=16")], "").len(), 1);
    assert_eq!(
        found(&[], "[profile.judged]\ncodegen-units = 16\n"),
        ["/h/.cargo/config.toml:2: codegen-units = 16"]
    );
    assert_eq!(found(&[], "profile.judged.codegen-units = 16\n").len(), 1);
    assert_eq!(
        found(
            &[
                ("RUSTFLAGS", "-C target-cpu=native"),
                ("CARGO_PROFILE_RELEASE_CODEGEN_UNITS", "16")
            ],
            "[profile.release]\ncodegen-units = 16\n"
        ),
        Vec::<String>::new()
    );
}

#[test]
fn the_configs_read_are_each_directory_up_and_cargos_home() {
    let paths = build_judged::config_paths(
        std::path::Path::new("/w/crates/perf"),
        std::path::Path::new("/h/.cargo"),
    );
    for expected in [
        "/w/crates/perf/.cargo/config.toml",
        "/w/.cargo/config.toml",
        "/w/.cargo/config",
        "/h/.cargo/config.toml",
    ] {
        assert!(
            paths.contains(&std::path::PathBuf::from(expected)),
            "{expected} in {paths:?}"
        );
    }
}

#[test]
fn the_profile_is_named_from_out_dir_and_an_unknown_layout_is_refused() {
    let named = |out: &str| build_judged::profile_of(std::path::Path::new(out));
    assert_eq!(
        named("/t/judged/build/vapors-perf-1a/out").as_deref(),
        Ok("judged")
    );
    assert_eq!(
        named("/t/x86_64-unknown-linux-gnu/release/build/p-1/out").as_deref(),
        Ok("release")
    );
    let refused = named("/t/judged/build/vapors-perf/1a/out").expect_err("build-dir layout");
    assert!(refused.contains("cannot be named"), "{refused}");
}

#[test]
fn only_a_judged_build_is_refused_for_an_override() {
    // Given an override in the environment
    let env = vec![(
        "CARGO_PROFILE_JUDGED_CODEGEN_UNITS".to_owned(),
        "16".to_owned(),
    )];

    // Then a release build goes on, and a judged one is refused naming it
    assert_eq!(build_judged::refusal("release", &env, &[]), Ok(()));
    let refused = build_judged::refusal("judged", &env, &[]).expect_err("overridden");
    assert!(
        refused.contains("CARGO_PROFILE_JUDGED_CODEGEN_UNITS=16"),
        "{refused}"
    );
    assert_eq!(build_judged::refusal("judged", &[], &[]), Ok(()));
}

#[test]
fn a_scene_run_names_the_profile_built_and_whether_it_is_the_shipped_one() {
    assert!(super::scene_header("release").starts_with("release (the shipped profile"));
    let other = super::scene_header("judged");
    assert!(other.starts_with("judged (not release"), "{other}");
}

#[test]
fn a_mount_run_names_the_profile_built_and_whether_it_is_the_shipped_one() {
    assert_eq!(
        super::shipped_header("release"),
        "release (the shipped profile)"
    );
    let other = super::shipped_header("judged");
    assert!(other.starts_with("judged (not release"), "{other}");
}
