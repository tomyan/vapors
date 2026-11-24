//! The gate's steps see no GPU device where there is one to hide.

use super::*;

fn dri_with(name: &str, entries: &[&str]) -> crate::scratch::Scratch {
    let dir = crate::scratch::dir(&format!("dri-{name}"));
    for entry in entries {
        std::fs::write(dir.join(entry), "").expect("entry");
    }
    dir
}

#[test]
fn every_step_runs_with_dev_dri_hidden() {
    assert_eq!(
        HIDE_GPU_DEVICE,
        [
            "--dev-bind",
            "/",
            "/",
            "--tmpfs",
            "/dev/dri",
            "--die-with-parent"
        ]
    );
}

#[test]
fn a_render_node_is_there_or_not_and_an_unreadable_directory_is_refused() {
    // Given
    let card = dri_with("card", &["card0"]);
    let render = dri_with("render", &["card0", "renderD128"]);
    let locked = dri_with("locked", &[]);
    std::fs::set_permissions(
        &*locked,
        std::os::unix::fs::PermissionsExt::from_mode(0o000),
    )
    .expect("lock");

    // Then
    assert_eq!(has_render_node(&card.join("absent")), Ok(false));
    assert_eq!(has_render_node(&card), Ok(false));
    assert_eq!(has_render_node(&render), Ok(true));
    let refusal = has_render_node(&locked).expect_err("unreadable");
    assert!(refusal.contains("Permission denied"), "{refusal}");
    assert!(refusal.contains("hide_gpu_device"), "{refusal}");
    std::fs::set_permissions(
        &*locked,
        std::os::unix::fs::PermissionsExt::from_mode(0o755),
    )
    .expect("unlock");
}

#[test]
fn a_machine_without_bwrap_is_refused_naming_the_fix() {
    // When
    let refusal = check_with("no-such-bwrap-for-this-test").expect_err("refused");

    // Then
    assert!(refusal.contains("no-such-bwrap-for-this-test"), "{refusal}");
    assert!(refusal.contains("fix: install bubblewrap"), "{refusal}");
    assert!(refusal.contains("hide_gpu_device = \"false\""), "{refusal}");
}

/// A stand-in for bwrap at a path in a scratch directory: a script that
/// writes its arguments to `bwrap.argv` beside it, prints `listed`, and
/// exits with `code`.
fn fake_bwrap(name: &str, listed: &str, code: i32) -> (crate::scratch::Scratch, String) {
    let dir = crate::scratch::dir(name);
    let program = dir.join("bwrap");
    std::fs::write(
        &program,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$0.argv\"\nprintf '%s' '{listed}'\nexit {code}\n"
        ),
    )
    .expect("written");
    let executable = std::os::unix::fs::PermissionsExt::from_mode(0o755);
    std::fs::set_permissions(&program, executable).expect("mode");
    (dir, program.display().to_string())
}

#[test]
fn a_bwrap_that_lists_nothing_hides_dev_dri_and_one_that_lists_or_fails_does_not() {
    // Given: bwrap's answers given, not asked, as an API builder's
    // sandbox has no bwrap that can run (its real twin is
    // host_where_there_is_a_render_node_bwrap_hides_dev_dri, the host tier)
    let (_empty, hides) = fake_bwrap("bwrap-hides", "", 0);
    let (_listed, shows) = fake_bwrap("bwrap-shows", "renderD128", 0);
    let (_failed, fails) = fake_bwrap("bwrap-fails", "", 1);

    // When
    let hidden = check_with(&hides);

    // Then: it was asked to hide /dev/dri, and its answers are read
    assert_eq!(hidden, Ok(()));
    let argv = std::fs::read_to_string(format!("{hides}.argv")).expect("the stand-in's arguments");
    assert!(argv.contains("--tmpfs\n/dev/dri\n"), "{argv}");
    assert!(
        check_with(&shows)
            .expect_err("listed")
            .contains("it listed `renderD128`")
    );
    assert!(
        check_with(&fails)
            .expect_err("failed")
            .contains("did not run with /dev/dri hidden")
    );
}

#[test]
fn host_where_there_is_a_render_node_bwrap_hides_dev_dri() {
    // Given: the dev box has one; GitHub's runner has none (and no bwrap),
    // and there the gate runs steps as they are. The real bwrap, so the
    // host tier (steps_tiers.rs HOST_TESTS); its unit twin is
    // a_bwrap_that_lists_nothing_hides_dev_dri_and_one_that_lists_or_fails_does_not
    if !has_render_node(Path::new("/dev/dri")).expect("readable") {
        assert_eq!(
            wrap(&Config::parse("").expect("empty")),
            Ok(Wrap::NoRenderNode)
        );
        return;
    }

    // When
    let checked = check_with("bwrap");

    // Then
    assert_eq!(checked, Ok(()));
}

#[test]
fn the_steps_hide_the_device_only_where_there_is_one() {
    // Given
    let unset = Config::parse("").expect("empty");
    let (card, render) = (
        dri_with("w-card", &["card0"]),
        dri_with("w-render", &["renderD128"]),
    );

    // Then: a missing bwrap is refused only where there is a render node
    assert_eq!(
        wrap_for(&unset, &card, "no-such-bwrap-for-this-test"),
        Ok(Wrap::NoRenderNode)
    );
    let refusal = wrap_for(&unset, &render, "no-such-bwrap-for-this-test").expect_err("refused");
    assert!(refusal.contains("holds: renderD128"), "{refusal}");
}

#[test]
fn the_off_switch_runs_steps_as_they_are_and_another_value_is_refused() {
    // Given
    let off = Config::parse("[gate]\nhide_gpu_device = \"false\"\n").expect("off");
    let odd = Config::parse("[gate]\nhide_gpu_device = \"no\"\n").expect("odd");

    // Then
    assert_eq!(wrap(&off), Ok(Wrap::SwitchedOff));
    let refusal = wrap(&odd).expect_err("refused");
    assert!(refusal.contains("fix: .config/factory.toml"), "{refusal}");
}

#[test]
fn only_an_unsandboxed_gate_has_a_notice() {
    assert_eq!(Wrap::HideGpuDevice.notice(), None);
    assert!(
        Wrap::NoRenderNode
            .notice()
            .is_some_and(|n| n.contains("no render node"))
    );
    assert!(
        Wrap::SwitchedOff
            .notice()
            .is_some_and(|n| n.contains("hide_gpu_device"))
    );
}

#[test]
fn a_sandbox_check_past_its_deadline_fails_naming_the_check() {
    // Given: a bwrap that does not end
    let dir = crate::scratch::dir("bwrap-stuck");
    let program = dir.join("bwrap");
    std::fs::write(&program, "#!/bin/sh\nexec sleep 30\n").expect("written");
    let executable = std::os::unix::fs::PermissionsExt::from_mode(0o755);
    std::fs::set_permissions(&program, executable).expect("mode");

    // When
    let refusal = check_within(
        &program.display().to_string(),
        std::time::Duration::from_millis(100),
    )
    .expect_err("past its deadline");

    // Then
    assert!(
        refusal.contains("the gate's sandbox check")
            && refusal.contains("deadline")
            && refusal.contains("cargo xtask gate"),
        "{refusal}"
    );
}
