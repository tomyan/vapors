//! The count checks, on values given, with no counter opened.

use std::collections::BTreeMap;
use std::time::Duration;

use super::{phases_counted, rose_by_at_least, whole_time};

#[test]
fn a_counter_that_ran_the_whole_time_is_accepted() {
    assert_eq!(whole_time(5_000, 5_000), Ok(()));
}

#[test]
fn a_counter_scheduled_off_for_part_of_the_time_is_refused_naming_both_times() {
    // When
    let error = whole_time(5_000, 4_000).expect_err("multiplexed");

    // Then
    assert!(error.contains("time_enabled 5000"), "{error}");
    assert!(error.contains("time_running 4000"), "{error}");
    assert!(error.contains("stop other perf users"), "{error}");
    assert!(error.contains("sysctl kernel.nmi_watchdog=0"), "{error}");
}

#[test]
fn a_count_that_rose_by_the_loop_is_accepted() {
    assert_eq!(rose_by_at_least(10, 100_010, 100_000), Ok(()));
}

#[test]
fn a_count_that_rose_less_than_the_loop_is_refused_naming_the_rise() {
    let error = rose_by_at_least(10, 5_010, 100_000).expect_err("short");
    assert!(error.contains("rose by 5000"), "{error}");
    assert!(error.contains("100000"), "{error}");
}

#[test]
fn a_count_that_fell_is_refused() {
    assert!(rose_by_at_least(500, 100, 1).is_err());
}

fn phases<T: Default>(names: &[&str]) -> BTreeMap<String, T> {
    names
        .iter()
        .map(|&n| (n.to_owned(), T::default()))
        .collect()
}

#[test]
fn every_timed_phase_counted_is_accepted() {
    let timed = phases::<Duration>(&["layout", "style"]);
    let counted = phases::<u64>(&["layout", "paint", "style"]);
    assert_eq!(phases_counted(&timed, &counted), Ok(()));
}

#[test]
fn a_timed_phase_without_a_count_is_refused_naming_it() {
    let timed = phases::<Duration>(&["layout", "paint", "style"]);
    let counted = phases::<u64>(&["layout"]);
    let error = phases_counted(&timed, &counted).expect_err("uncounted");
    assert!(error.ends_with("paint, style"), "{error}");
}
