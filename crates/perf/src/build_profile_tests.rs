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
