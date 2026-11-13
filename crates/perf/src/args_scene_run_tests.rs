//! `--scene-run`: alone, without counts.

use super::options;

fn parsed(args: &[&str]) -> Result<bool, String> {
    options(args.iter().map(|&arg| arg.to_owned())).map(|o| o.scene_run)
}

#[test]
fn scene_run_alone_is_the_60_s_run() {
    assert_eq!(parsed(&["--scene-run"]), Ok(true));
    assert_eq!(parsed(&[]), Ok(false));
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
