//! Which workloads exist, and the frames a big page runs for.

use super::{BIG_PAGE, settings_for, workloads};
use crate::run::Settings;

fn names() -> Vec<String> {
    workloads().into_iter().map(|(name, _)| name).collect()
}

#[test]
fn the_full_run_has_the_10000_row_edits_and_insert_at_three_sizes() {
    let names = names();
    for name in [
        "10000 rows, Idle",
        "10000 rows, Text",
        "10000 rows, Class",
        "10000 rows, HoveredClass",
        "1000 rows, Insert",
        "3000 rows, Insert",
        "10000 rows, Insert",
    ] {
        assert!(names.iter().any(|n| n == name), "{name} in {names:?}");
    }
}

#[test]
fn no_workload_is_named_twice() {
    let mut names = names();
    let count = names.len();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), count);
}

#[test]
fn a_big_page_runs_at_most_60_frames_after_10_warmup() {
    let given = Settings {
        warmup: 20,
        runs: 200,
    };
    let big = settings_for(Some(BIG_PAGE), given);
    assert_eq!((big.warmup, big.runs), (10, 60));
}

#[test]
fn a_smaller_request_is_kept_and_smaller_pages_and_the_racer_are_not_capped() {
    let given = Settings {
        warmup: 2,
        runs: 30,
    };
    let kept = settings_for(Some(BIG_PAGE), given);
    assert_eq!((kept.warmup, kept.runs), (2, 30));
    for rows in [Some(BIG_PAGE - 1), None] {
        let uncapped = settings_for(
            rows,
            Settings {
                warmup: 20,
                runs: 200,
            },
        );
        assert_eq!((uncapped.warmup, uncapped.runs), (20, 200));
    }
}
