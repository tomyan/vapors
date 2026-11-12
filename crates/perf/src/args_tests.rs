//! The command line: counts within the GPU clock's capacity, and
//! runs and warmup, `--only` with a value, `--instructions`, no other flag.

use super::{MAX_RUNS, options};

fn parsed(args: &[&str]) -> Result<(usize, usize), String> {
    options(args.iter().map(|&arg| arg.to_owned())).map(|o| (o.settings.runs, o.settings.warmup))
}

#[test]
fn no_arguments_give_200_runs_after_20_warmup() {
    assert_eq!(parsed(&[]), Ok((200, 20)));
}

#[test]
fn runs_and_warmup_are_read_in_that_order() {
    assert_eq!(parsed(&["100", "5"]), Ok((100, 5)));
}

#[test]
fn runs_from_1_to_the_clock_capacity_are_accepted() {
    assert_eq!(parsed(&["1"]), Ok((1, 20)));
    assert_eq!(parsed(&[&MAX_RUNS.to_string()]), Ok((MAX_RUNS, 20)));
}

#[test]
fn runs_outside_1_to_the_clock_capacity_are_refused_naming_the_range() {
    for runs in ["0", &(MAX_RUNS + 1).to_string()] {
        let error = parsed(&[runs]).expect_err(runs);
        assert!(error.contains("give 1 to 2048"), "{error}");
    }
}

#[test]
fn a_third_argument_is_refused_naming_it() {
    let error = parsed(&["10", "2", "3"]).expect_err("three arguments");
    assert!(error.starts_with("unexpected argument 3: "), "{error}");
}

#[test]
fn a_word_is_refused_as_not_a_count() {
    let error = parsed(&["many"]).expect_err("a word");
    assert!(error.starts_with("many is not a count: "), "{error}");
}

fn full(args: &[&str]) -> Result<super::Options, String> {
    options(args.iter().map(|&arg| arg.to_owned()))
}

#[test]
fn only_and_instructions_are_read_among_the_counts() {
    let o = full(&["--instructions", "50", "--only", "3000 rows, Class", "5"]).expect("parsed");
    assert_eq!((o.settings.runs, o.settings.warmup), (50, 5));
    assert_eq!(o.only.as_deref(), Some("3000 rows, Class"));
    assert!(o.instructions);
}

#[test]
fn without_flags_there_is_no_filter_and_times_are_reported() {
    let o = full(&[]).expect("parsed");
    assert_eq!((o.only, o.instructions), (None, false));
}

#[test]
fn an_unknown_flag_is_refused_naming_it() {
    let error = full(&["--instruction"]).expect_err("typo");
    assert!(error.starts_with("unknown flag --instruction: "), "{error}");
}

#[test]
fn only_without_a_value_is_refused() {
    let error = full(&["10", "--only"]).expect_err("no value");
    assert!(error.starts_with("--only needs a value: "), "{error}");
}

#[test]
fn a_repeated_only_is_refused_naming_it() {
    let error = full(&["--only", "a", "--only", "b"]).expect_err("twice");
    assert!(error.starts_with("--only given twice: "), "{error}");
    assert!(error.contains("--only <substring> (once)"), "{error}");
}

#[test]
fn scaling_names_a_change_and_counts_still_come_before_or_after_it() {
    let o = full(&["60", "--scaling", "Insert", "10"]).expect("parsed");
    assert!(matches!(o.scaling, Some(crate::rows::RowChange::Insert)));
    assert_eq!((o.settings.runs, o.settings.warmup), (60, 10));
    assert!(full(&[]).expect("parsed").scaling.is_none());
}

#[test]
fn scaling_without_a_change_or_with_an_unknown_one_is_refused_naming_it() {
    let error = full(&["--scaling"]).expect_err("no value");
    assert!(error.starts_with("--scaling needs a change: "), "{error}");
    let error = full(&["--scaling", "Hover"]).expect_err("unknown");
    assert!(
        error.starts_with("--scaling Hover is not a change: "),
        "{error}"
    );
    assert!(
        error.contains("Idle|Text|Class|HoveredClass|Insert"),
        "{error}"
    );
}

#[test]
fn scaling_given_twice_or_with_only_is_refused() {
    let error = full(&["--scaling", "Text", "--scaling", "Class"]).expect_err("twice");
    assert!(error.starts_with("--scaling given twice: "), "{error}");
    let error = full(&["--scaling", "Text", "--only", "Text"]).expect_err("with only");
    assert!(
        error.starts_with("--scaling and --only cannot be combined: "),
        "{error}"
    );
}

#[test]
fn scaling_with_instructions_is_refused() {
    // Given: --scaling always counts instructions
    // When
    let error = full(&["--scaling", "Text", "--instructions"]).expect_err("with instructions");

    // Then
    assert!(
        error.starts_with("--scaling and --instructions cannot be combined"),
        "{error}"
    );
}

#[test]
fn mount_is_read_as_a_flag() {
    assert!(full(&["--mount", "5"]).expect("parsed").mount);
    assert!(!full(&[]).expect("parsed").mount);
}

#[test]
fn mount_with_only_instructions_or_scaling_is_refused_naming_it() {
    for other in [
        &["--only", "x"][..],
        &["--instructions"],
        &["--scaling", "Idle"],
    ] {
        let mut args = vec!["--mount"];
        args.extend_from_slice(other);
        let error = full(&args).expect_err("combined");
        assert!(
            error.starts_with(&format!("--mount and {} cannot be combined", other[0])),
            "{error}"
        );
    }
}

#[test]
fn mount_memory_takes_a_row_count() {
    assert_eq!(
        full(&["--mount-memory", "3000"])
            .expect("parsed")
            .mount_memory,
        Some(3000)
    );
    assert_eq!(full(&[]).expect("parsed").mount_memory, None);
}

#[test]
fn mount_memory_accepts_zero_rows_as_the_baseline() {
    let parsed = full(&["--mount-memory", "0"]).expect("parsed");
    assert_eq!(parsed.mount_memory, Some(0));
}

#[test]
fn mount_memory_without_a_count_or_with_a_word_is_refused() {
    for args in [&["--mount-memory"][..], &["--mount-memory", "many"]] {
        let error = full(args).expect_err("bad count");
        assert!(
            error.starts_with("--mount-memory needs a row count"),
            "{error}"
        );
    }
}

#[test]
fn mount_memory_with_mount_or_the_other_modes_is_refused_naming_the_flag() {
    for other in [
        &["--mount"][..],
        &["--only", "x"],
        &["--instructions"],
        &["--scaling", "Idle"],
    ] {
        let mut args = vec!["--mount-memory", "10"];
        args.extend_from_slice(other);
        let error = full(&args).expect_err("combined");
        assert!(
            error.starts_with(&format!(
                "--mount-memory and {} cannot be combined",
                other[0]
            )),
            "{error}"
        );
    }
}
