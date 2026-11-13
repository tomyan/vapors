//! `--off-path`: read with `--only`, `--instructions` and the counts;
//! refused with the modes that replace the full run.

use super::{WorkloadSet, options};

fn full(args: &[&str]) -> Result<super::Options, String> {
    options(args.iter().map(|&arg| arg.to_owned()))
}

#[test]
fn off_path_is_read_with_only_instructions_and_the_counts() {
    let o = full(&[
        "60",
        "--off-path",
        "--only",
        "code blocks",
        "--instructions",
        "10",
    ])
    .expect("parsed");
    assert_eq!(o.set, WorkloadSet::OffPath);
    assert!(o.instructions);
    assert_eq!(o.only.as_deref(), Some("code blocks"));
    assert_eq!((o.settings.runs, o.settings.warmup), (60, 10));
    assert_eq!(full(&[]).expect("parsed").set, WorkloadSet::Main);
}

#[test]
fn off_path_with_a_mode_that_replaces_the_full_run_is_refused_naming_both() {
    for mode in [
        &["--scaling", "Text"][..],
        &["--mount"],
        &["--mount-memory", "1000"],
        &["--scene-run"],
    ] {
        let args = [&["--off-path"][..], mode].concat();
        let error = full(&args).expect_err("refused");
        let expected = format!("--off-path and {} cannot be combined: ", mode[0]);
        assert!(error.starts_with(&expected), "{error}");
    }
}
