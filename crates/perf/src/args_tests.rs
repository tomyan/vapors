//! The command line: counts within the GPU clock's capacity, and
//! nothing more than runs and warmup.

use super::{MAX_RUNS, settings};

fn parsed(args: &[&str]) -> Result<(usize, usize), String> {
    settings(args.iter().map(|&arg| arg.to_owned())).map(|s| (s.runs, s.warmup))
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
