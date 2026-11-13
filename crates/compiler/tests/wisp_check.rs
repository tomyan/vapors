//! The `wisp-check` binary: exit 0 with the component's name, or exit
//! 1 with every refusal as `file:line:col: message`. Native only.

#![cfg(not(target_family = "wasm"))]

#[path = "common/build.rs"]
mod build;

use std::process::{Command, Output};

use build::{COUNTER, dirs, write};

fn wisp_check(args: &[&std::path::Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wisp-check"))
        .args(args)
        .output()
        .expect("run wisp-check")
}

#[test]
fn a_component_that_compiles_exits_zero_and_names_it() {
    // Given
    let (src, _) = dirs("check_ok");
    write(&src, "counter.wisp", COUNTER);

    // When
    let output = wisp_check(&[&src.join("counter.wisp")]);

    // Then
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("`Counter`"),
        "{output:?}"
    );
}

#[test]
fn a_codegen_refusal_exits_one_with_its_position() {
    // Given: parses, but a signal without a type cannot become a field
    let (src, _) = dirs("check_codegen_refusal");
    write(
        &src,
        "a.wisp",
        "<script>\nlet n = Signal::new(0);\n</script>\n<p>{n}</p>",
    );

    // When
    let output = wisp_check(&[&src.join("a.wisp")]);

    // Then
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let expected = format!("{}:2:1: ", src.join("a.wisp").display());
    assert!(
        String::from_utf8_lossy(&output.stderr).starts_with(&expected),
        "{output:?}"
    );
}

const GLOBAL_RULE: &str =
    "<style>\np { color: red; }\n:global(body) { margin: 0; }\n</style>\n<p>Card</p>";

#[test]
fn a_top_level_global_passes_the_scoped_reading_and_fails_the_global_one() {
    // Given: what compile_dir accepts and compile_dir_global refuses
    let (src, _) = dirs("check_global_flag");
    write(&src, "card.wisp", GLOBAL_RULE);
    let file = src.join("card.wisp");

    // When
    let scoped = wisp_check(&[&file]);
    let global = Command::new(env!("CARGO_BIN_EXE_wisp-check"))
        .arg("--global")
        .arg(&file)
        .output()
        .expect("run wisp-check");

    // Then
    assert_eq!(scoped.status.code(), Some(0), "{scoped:?}");
    assert!(
        String::from_utf8_lossy(&scoped.stdout).contains("(scoped style)"),
        "{scoped:?}"
    );
    assert_eq!(global.status.code(), Some(1), "{global:?}");
    let expected = format!("{}:3:1: ", file.display());
    assert!(
        String::from_utf8_lossy(&global.stderr).starts_with(&expected),
        "{global:?}"
    );
}

#[test]
fn no_argument_exits_two_with_usage() {
    // When
    let output = wisp_check(&[]);

    // Then
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("usage: wisp-check"),
        "{output:?}"
    );
}
