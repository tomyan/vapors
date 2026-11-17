use super::{Probe, Ran, SENTINEL, prove, shell};

fn probe(script: &str) -> Probe {
    Probe {
        name: "test escape",
        script: script.into(),
        lands_at: None,
    }
}

/// An inside where every command fails, as a working sandbox's escapes do.
fn refusing(_: &str) -> Result<Ran, String> {
    shell(&format!("echo '{SENTINEL}'; exit 1"))
}

/// An inside that runs the command as it is: no sandbox at all.
fn open(script: &str) -> Result<Ran, String> {
    shell(&format!("echo '{SENTINEL}'; {script}"))
}

/// An inside that never started: bwrap refused before running anything.
fn never_started(_: &str) -> Result<Ran, String> {
    shell("echo 'bwrap: Can not mount tmpfs' >&2; exit 1")
}

#[test]
fn a_fence_holds_when_its_escape_works_outside_and_fails_inside() {
    let held = prove(&[probe("true")], &refusing);
    assert_eq!(held, Ok(vec!["fence held: test escape".to_owned()]));
}

#[test]
fn an_escape_that_works_inside_breaks_the_run() {
    let broken = prove(&[probe("true")], &open).expect_err("refused");
    assert!(broken.starts_with("fence broken, test escape:"), "{broken}");
}

#[test]
fn a_probe_that_fails_outside_too_cannot_prove_its_fence() {
    let blind = prove(&[probe("false")], &refusing).expect_err("refused");
    assert!(
        blind.starts_with("probe cannot show an escape, test escape:"),
        "{blind}"
    );
}

#[test]
fn a_write_that_lands_outside_breaks_the_run_though_the_command_failed() {
    // Given: the escape writes a file and exits 1 inside
    let dir = tempfile::tempdir().expect("a scratch directory");
    let file = dir.path().join("landed");
    let write = format!("echo x > '{}'", file.display());
    let landing = Probe {
        name: "test write",
        script: write.clone(),
        lands_at: Some(file.clone()),
    };
    let writes_and_fails = |_: &str| shell(&format!("echo '{SENTINEL}'; {write}; exit 1"));

    // When
    let broken = prove(&[landing], &writes_and_fails);

    // Then
    assert!(
        broken
            .expect_err("refused")
            .starts_with("fence broken, test write:")
    );
}

#[test]
fn a_sandbox_that_never_started_is_an_error_not_a_held_fence() {
    let refused = prove(&[probe("true")], &never_started).expect_err("refused");
    assert!(
        refused.starts_with("the sandbox did not run probe"),
        "{refused}"
    );
}

#[test]
fn the_canary_is_set_outside_so_its_probe_can_show_the_environment_escaping() {
    let outside = shell(&format!(
        "test \"${}\" = {}",
        super::CANARY.0,
        super::CANARY.1
    ));
    assert!(outside.expect("ran").success);
}
