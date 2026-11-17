use std::path::Path;

use super::inherited;

/// An fdinfo directory with one entry per (fd, flags).
fn fdinfo(entries: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a scratch directory");
    for (fd, flags) in entries {
        std::fs::write(
            dir.path().join(fd),
            format!("pos:\t0\nflags:\t{flags}\nmnt_id:\t1\n"),
        )
        .expect("written");
    }
    dir
}

#[test]
fn a_descriptor_above_2_without_close_on_exec_is_named() {
    // Given: stdio, one closed on exec (02000002), one not (0100002)
    let dir = fdinfo(&[
        ("0", "02"),
        ("1", "01"),
        ("2", "01"),
        ("3", "02000002"),
        ("4", "0100002"),
    ]);

    // When
    let open = inherited(dir.path(), Path::new("/nonexistent")).expect("read");

    // Then
    assert_eq!(open.len(), 1, "{open:?}");
    assert!(open[0].starts_with("fd 4 ("), "{open:?}");
}

#[test]
fn an_fdinfo_entry_without_flags_is_refused() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    std::fs::write(dir.path().join("5"), "pos:\t0\n").expect("written");
    let refused = inherited(dir.path(), Path::new("/nonexistent")).expect_err("refused");
    assert!(refused.contains("has no octal `flags:` line"), "{refused}");
}
