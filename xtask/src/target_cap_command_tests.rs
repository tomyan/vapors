//! The cap measures the directory the command it guards builds into:
//! the command's own `CARGO_TARGET_DIR` over xtask's.

use std::ffi::OsString;
use std::process::Command;

use super::*;

#[test]
fn a_command_s_own_cargo_target_dir_is_the_one_capped() {
    // Given
    let mut command = Command::new("cargo");
    command.env("CARGO_TARGET_DIR", "/scratch/target");

    // When
    let capped = command_target_dir(&command);

    // Then
    assert_eq!(capped, Some(OsString::from("/scratch/target")));
}

#[test]
fn a_command_that_removes_cargo_target_dir_is_capped_on_the_workspace_target() {
    // Given
    let mut command = Command::new("cargo");
    command.env_remove("CARGO_TARGET_DIR");

    // When
    let capped = command_target_dir(&command);

    // Then
    assert_eq!(capped, None);
}

#[test]
fn a_command_that_leaves_cargo_target_dir_alone_is_capped_on_the_inherited_one() {
    // Given
    let command = Command::new("cargo");

    // When
    let capped = command_target_dir(&command);

    // Then
    assert_eq!(capped, std::env::var_os("CARGO_TARGET_DIR"));
}
