use super::{allowed, checked};

fn default() -> Vec<String> {
    allowed(&[]).expect("the defaults")
}

/// `numstat` rows as `git diff --numstat -z` writes them.
fn z(rows: &[&str]) -> String {
    rows.iter().flat_map(|row| [*row, "\0"]).collect()
}

#[test]
fn a_branch_inside_the_allowed_paths_and_cap_passes_with_its_line_count() {
    let numstat = z(&["10\t2\tcrates/scene/src/a.rs", "5\t0\tdocs/plans/x.md"]);
    assert_eq!(checked(&numstat, "", &default()), Ok(17));
}

#[test]
fn a_file_that_changes_what_runs_outside_is_refused_unless_named() {
    // Given
    let numstat = z(&["1\t0\txtask/src/main.rs", "1\t0\tcrates/a/build.rs"]);

    // When
    let refused = checked(&numstat, "", &default());
    let named = checked(
        &numstat,
        "",
        &allowed(&["xtask/".to_owned()]).expect("named"),
    );

    // Then
    assert_eq!(
        refused,
        Err("xtask/src/main.rs is outside crates/, docs/".to_owned())
    );
    assert_eq!(named, Ok(2));
}

#[test]
fn a_binary_file_is_refused() {
    let refused = checked(&z(&["-\t-\tcrates/a/image.png"]), "", &default());
    assert_eq!(refused, Err("crates/a/image.png is binary".to_owned()));
}

#[test]
fn more_than_the_cap_is_refused_naming_the_count() {
    let refused =
        checked(&z(&["300\t101\tcrates/a/src/lib.rs"]), "", &default()).expect_err("refused");
    assert!(
        refused.starts_with("401 lines changed, over 400"),
        "{refused}"
    );
}

#[test]
fn a_symlink_or_submodule_is_refused_though_its_path_is_allowed() {
    // Given
    let numstat = z(&["1\t0\tcrates/a/link", "1\t0\tcrates/b"]);
    let raw = z(&[
        ":000000 120000 0000 1111 A",
        "crates/a/link",
        ":000000 160000 0000 2222 A",
        "crates/b",
    ]);

    // When
    let refused = checked(&numstat, &raw, &default()).expect_err("refused");

    // Then
    assert_eq!(
        refused,
        "crates/a/link is a symlink; crates/b is a submodule"
    );
}

#[test]
fn a_path_with_a_dot_dot_part_or_not_utf8_is_refused_though_it_starts_allowed() {
    for path in [
        "crates/../xtask/src/main.rs",
        "crates/.git/config",
        "crates/\u{fffd}.rs",
    ] {
        let refused = checked(&z(&[&format!("1\t0\t{path}")]), "", &default()).expect_err(path);
        assert!(
            refused.contains(path) || refused.contains(&format!("{path:?}")),
            "{refused}"
        );
    }
}

#[test]
fn a_path_git_would_quote_is_read_as_its_bytes() {
    let numstat = z(&["1\t0\tcrates/a/naïve \"x\".rs"]);
    assert_eq!(checked(&numstat, "", &default()), Ok(1));
}

#[test]
fn an_allowed_path_that_is_not_a_relative_directory_is_refused() {
    for named in ["", "/", "xtask", "/etc/", "crates/../xtask/"] {
        let refused = allowed(&[named.to_owned()]).expect_err(named);
        assert!(refused.starts_with("--allow"), "{refused}");
    }
}

#[test]
fn a_record_of_another_shape_is_refused() {
    let refused = checked("not numstat\0", "", &default()).expect_err("refused");
    assert!(
        refused.starts_with("a `git diff --numstat -z` record"),
        "{refused}"
    );
}
