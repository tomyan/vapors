//! A committed oracle answer compared with Chromium's.

use super::*;

/// A file holding `contents`, or no file at all, removed when dropped.
fn file(test: &str, contents: Option<&str>) -> tempfile::TempPath {
    let made = tempfile::Builder::new()
        .prefix(&format!("ax-compare-{test}-"))
        .suffix(".json")
        .tempfile();
    let path = made
        .expect("a scratch file under the temp dir")
        .into_temp_path();
    match contents {
        Some(contents) => std::fs::write(&path, contents).expect("file"),
        None => std::fs::remove_file(&path).expect("no file"),
    }
    path
}

#[test]
fn an_answer_is_the_same_missing_malformed_or_different_at_its_first_line() {
    let ours = "{\n  \"a\": 1,\n  \"b\": 2\n}\n";
    assert_eq!(
        compare(&file("same", Some(ours)), ours),
        Ok(Committed::Same)
    );
    assert_eq!(
        compare(&file("missing", None), ours),
        Ok(Committed::Missing)
    );
    assert!(matches!(
        compare(&file("malformed", Some("{ nope")), ours),
        Ok(Committed::Malformed(_))
    ));
    let different = compare(
        &file("different", Some("{\n  \"a\": 1,\n  \"b\": 3\n}\n")),
        ours,
    );
    assert_eq!(
        different,
        Ok(Committed::Different {
            line: 3,
            committed: "\"b\": 3".into(),
            chromium: "\"b\": 2".into()
        })
    );
}

#[test]
fn each_outcome_is_reported_with_its_fix() {
    assert!(
        Committed::Missing
            .report("buttons")
            .contains("buttons: no committed answer")
    );
    let different = Committed::Different {
        line: 3,
        committed: "a".into(),
        chromium: "b".into(),
    };
    assert!(
        different
            .report("buttons")
            .contains("buttons: line 3: committed `a`, Chromium `b`")
    );
}

#[test]
fn a_reader_never_sees_part_of_a_file_being_rewritten() {
    // Given
    let dir = tempfile::tempdir().expect("a scratch directory");
    let path = dir.path().join("tall.png");
    let bytes = vec![7_u8; 4096];
    write_whole(&path, &bytes).expect("first write");

    // When
    let writer = {
        let (path, bytes) = (path.clone(), bytes.clone());
        std::thread::spawn(move || {
            for _ in 0..2000 {
                write_whole(&path, &bytes).expect("rewrite");
            }
        })
    };
    let lengths: Vec<usize> = (0..2000)
        .map(|_| std::fs::read(&path).expect("read").len())
        .collect();
    writer.join().expect("writer");

    // Then
    let short = lengths.iter().filter(|&&n| n != bytes.len()).count();
    assert_eq!(short, 0, "{short} of 2000 reads saw part of the file");
}
