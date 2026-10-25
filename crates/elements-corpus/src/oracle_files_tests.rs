//! A committed oracle answer compared with Chromium's.

use super::*;

fn file(test: &str, contents: Option<&str>) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("ax-compare-{test}-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    if let Some(contents) = contents {
        std::fs::write(&path, contents).expect("file");
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
