//! `cargo xtask dev` as a process: without trunk on PATH it fails at
//! once naming trunk and the fix, before touching tailscale or a port.

use std::process::Command;

#[test]
fn dev_without_trunk_on_path_fails_naming_it() {
    // Given: a PATH that holds the system tools but not cargo's bin dir
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");

    // When
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("dev")
        .current_dir(root)
        .env("PATH", "/nonexistent")
        .output()
        .expect("xtask runs");

    // Then
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("FAIL dev: trunk is not on PATH; fix: `cargo xtask setup`"),
        "{text}"
    );
    assert!(text.contains("reproduce: cargo xtask dev"), "{text}");
}
