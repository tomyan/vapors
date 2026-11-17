//! The build directory a step names (`step_target_dir`): its own
//! `CARGO_TARGET_DIR`, taken from the step's directory when relative, as
//! cargo takes it there.

use std::path::{Path, PathBuf};

use super::*;

fn env(value: &str) -> Vec<(String, String)> {
    vec![("CARGO_TARGET_DIR".to_owned(), value.to_owned())]
}

#[test]
fn a_step_s_absolute_cargo_target_dir_is_its_build_directory() {
    // When
    let dir = step_target_dir(&env("/scratch/target"), Path::new("/work/crates/x"));

    // Then
    assert_eq!(dir, Some(PathBuf::from("/scratch/target")));
}

#[test]
fn a_relative_cargo_target_dir_is_taken_from_the_step_s_directory() {
    // When
    let dir = step_target_dir(&env("target"), Path::new("/work/crates/x"));

    // Then
    assert_eq!(dir, Some(PathBuf::from("/work/crates/x/target")));
}

#[test]
fn a_step_that_names_no_cargo_target_dir_or_an_empty_one_has_none() {
    // When
    let unset = step_target_dir(&[], Path::new("/work"));
    let empty = step_target_dir(&env(""), Path::new("/work"));

    // Then
    assert_eq!((unset, empty), (None, None));
}
