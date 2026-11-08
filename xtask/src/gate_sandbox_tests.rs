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

#[test]
fn where_there_is_a_render_node_bwrap_hides_dev_dri() {
    // Given: the dev box has one; GitHub's runner has none (and no bwrap),
    // and there the gate runs steps as they are
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
