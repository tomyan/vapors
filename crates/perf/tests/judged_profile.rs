//! A `--scaling` run of a vapors-perf built with any profile but
//! `judged` is refused before it opens a GPU or times anything: this
//! test's own build (the test profile, in target/debug) is one.

use std::process::Command;

#[test]
fn a_scaling_run_not_built_judged_refuses_naming_the_build_to_use() {
    // When
    let output = Command::new(env!("CARGO_BIN_EXE_vapors-perf"))
        .args(["60", "10", "--scaling", "all"])
        .output()
        .expect("vapors-perf runs");

    // Then
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{said}");
    assert!(said.contains("not judged"), "{said}");
    assert!(
        said.contains("cargo build --profile judged -p vapors-perf"),
        "{said}"
    );
}
