//! `--counters`: read among the counts, and refused with
//! `--instructions` and with the modes that replace the full run.

use super::Measure;
use crate::args::options;

fn full(args: &[&str]) -> Result<crate::args::Options, String> {
    options(args.iter().map(|&arg| arg.to_owned()))
}

#[test]
fn counters_is_read_with_only_off_path_and_the_counts() {
    let o = full(&["20", "--counters", "--off-path", "--only", "grid", "5"]).expect("parsed");
    assert_eq!(o.measure, Measure::Counters);
    assert_eq!((o.settings.runs, o.settings.warmup), (20, 5));
}

#[test]
fn counters_with_instructions_is_refused_either_way_round() {
    for args in [
        ["--counters", "--instructions"],
        ["--instructions", "--counters"],
    ] {
        let error = full(&args).expect_err("both");
        assert!(
            error.starts_with("--instructions and --counters cannot be combined"),
            "{error}"
        );
    }
}

#[test]
fn counters_with_scaling_or_a_mount_mode_is_refused_naming_counters() {
    let error = full(&["--scaling", "Text", "--counters"]).expect_err("scaling");
    assert!(
        error.starts_with("--scaling and --counters cannot be combined"),
        "{error}"
    );
    let error = full(&["--counters", "--mount"]).expect_err("mount");
    assert!(
        error.starts_with("--mount and --counters cannot be combined"),
        "{error}"
    );
}
