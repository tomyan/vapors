use super::{allowed, checked};

fn default() -> Vec<String> {
    allowed(&[])
}

#[test]
fn a_branch_inside_the_allowed_paths_and_cap_passes_with_its_line_count() {
    let numstat = "10\t2\tcrates/scene/src/a.rs\n5\t0\tdocs/plans/x.md\n";
    assert_eq!(checked(numstat, &default()), Ok(17));
}

#[test]
fn a_file_that_changes_what_runs_outside_is_refused_unless_named() {
    // Given
    let numstat = "1\t0\txtask/src/main.rs\n1\t0\tcrates/a/build.rs\n";

    // When
    let refused = checked(numstat, &default());
    let named = checked(numstat, &allowed(&["xtask/".to_owned()]));

    // Then
    assert_eq!(
        refused,
        Err("xtask/src/main.rs is outside crates/, docs/".to_owned())
    );
    assert_eq!(named, Ok(2));
}

#[test]
fn a_binary_file_is_refused() {
    let refused = checked("-\t-\tcrates/a/image.png\n", &default());
    assert_eq!(refused, Err("crates/a/image.png is binary".to_owned()));
}

#[test]
fn more_than_the_cap_is_refused_naming_the_count() {
    let refused = checked("300\t101\tcrates/a/src/lib.rs\n", &default()).expect_err("refused");
    assert!(
        refused.starts_with("401 lines changed, over 400"),
        "{refused}"
    );
}

#[test]
fn a_numstat_line_of_another_shape_is_refused() {
    let refused = checked("not numstat\n", &default()).expect_err("refused");
    assert!(
        refused.starts_with("a `git diff --numstat` line"),
        "{refused}"
    );
}
