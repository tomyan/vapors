//! `--scene-run`: alone, without counts.

use super::options;
use crate::scene_kind::SceneKind;

fn kind(args: &[&str]) -> Result<Option<SceneKind>, String> {
    options(args.iter().map(|&arg| arg.to_owned())).map(|o| o.scene_run)
}

fn parsed(args: &[&str]) -> Result<bool, String> {
    kind(args).map(|kind| kind.is_some())
}

#[test]
fn scene_run_alone_is_the_racers_60_s_run() {
    assert_eq!(kind(&["--scene-run"]), Ok(Some(SceneKind::Racer)));
    assert_eq!(kind(&[]), Ok(None));
}

#[test]
fn scene_run_takes_the_scene_named_after_it() {
    assert_eq!(
        kind(&["--scene-run", "one-scene"]),
        Ok(Some(SceneKind::OneScene))
    );
    assert_eq!(kind(&["--scene-run", "racer"]), Ok(Some(SceneKind::Racer)));
}

#[test]
fn scene_run_with_an_unknown_word_after_it_is_refused() {
    let error = parsed(&["--scene-run", "nope"]).expect_err("not a scene");
    assert!(error.contains("no scene is called `nope`"), "{error}");
}

#[test]
fn scene_run_with_a_count_is_refused_naming_it() {
    let error = parsed(&["--scene-run", "200"]).expect_err("a count");
    assert!(error.contains("takes no counts"), "{error}");
    assert!(error.contains("but 200 was given"), "{error}");
}

#[test]
fn scene_run_with_another_mode_or_selection_is_refused_naming_both() {
    for other in [&["--only", "racer"][..], &["--instructions"], &["--mount"]] {
        let mut args = other.to_vec();
        args.push("--scene-run");
        let error = parsed(&args).expect_err("two modes");
        assert!(
            error.starts_with(&format!("--scene-run and {} cannot", other[0])),
            "{error}"
        );
    }
}

#[test]
fn scene_run_given_twice_is_refused() {
    let error = parsed(&["--scene-run", "one-scene", "--scene-run"]).expect_err("twice");
    assert!(error.starts_with("--scene-run given twice"), "{error}");
}
