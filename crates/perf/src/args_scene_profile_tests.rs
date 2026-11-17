//! `--size` and `--profile-only`: parsed beside `--scene-run`, refused
//! without it or with a size that is not one.

use super::{SceneProfile, size_of};
use crate::args::options;

fn profile(args: &[&str]) -> Result<SceneProfile, String> {
    options(args.iter().map(|&arg| arg.to_owned())).map(|o| o.scene)
}

#[test]
fn a_scene_run_takes_a_size_and_profile_only() {
    // When
    let parsed = profile(&[
        "--scene-run",
        "racer",
        "--size",
        "1920x1080",
        "--profile-only",
    ]);

    // Then
    assert_eq!(
        parsed,
        Ok(SceneProfile {
            size: Some((1920, 1080)),
            profile_only: true
        })
    );
}

#[test]
fn a_scene_run_without_them_runs_at_its_own_size_and_is_judged() {
    assert_eq!(profile(&["--scene-run"]), Ok(SceneProfile::default()));
}

#[test]
fn a_size_that_is_not_two_sides_from_1_to_8192_is_refused_naming_it() {
    for value in [
        "1920",
        "1920x",
        "0x1080",
        "1920x8193",
        "wide x tall",
        "-1x2",
    ] {
        let error = size_of(Some(value)).expect_err(value);
        assert!(error.contains(&format!("--size {value}:")), "{error}");
    }
    let error = size_of(None).expect_err("no value");
    assert!(error.contains("(no value)"), "{error}");
}

#[test]
fn size_or_profile_only_without_a_scene_run_is_refused_naming_it() {
    for (args, flag) in [
        (&["--size", "640x480"][..], "--size"),
        (&["--profile-only"][..], "--profile-only"),
    ] {
        let error = profile(args).expect_err(flag);
        assert!(
            error.starts_with(&format!("{flag} changes a scene run")),
            "{error}"
        );
    }
}

#[test]
fn a_size_given_twice_is_refused() {
    let error = profile(&["--scene-run", "--size", "1x1", "--size", "2x2"]).expect_err("twice");
    assert!(error.contains("--size given twice"), "{error}");
}
