//! `--only` keeps workloads by substring and refuses a match of none.

use super::matching;

fn all() -> Vec<(String, ())> {
    ["3000 rows, Class", "3000 rows, Text", "racer, paused"]
        .map(|name| (name.to_owned(), ()))
        .to_vec()
}

fn names(chosen: &[(String, ())]) -> Vec<&str> {
    chosen.iter().map(|(name, ())| name.as_str()).collect()
}

#[test]
fn without_only_every_workload_runs() {
    let chosen = matching(all(), None).expect("all");
    assert_eq!(chosen.len(), 3);
}

#[test]
fn only_keeps_the_workloads_whose_name_contains_it() {
    let chosen = matching(all(), Some("3000 rows")).expect("two");
    assert_eq!(names(&chosen), ["3000 rows, Class", "3000 rows, Text"]);
}

#[test]
fn only_matching_nothing_is_refused_naming_every_workload() {
    let error = matching(all(), Some("nope")).expect_err("no match");
    assert!(
        error.contains("--only \"nope\" matches no workload"),
        "{error}"
    );
    assert!(error.contains("racer, paused"), "{error}");
}
