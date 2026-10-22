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
