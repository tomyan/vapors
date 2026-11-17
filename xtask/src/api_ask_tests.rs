use std::path::Path;
use std::time::Duration;

use super::{Asks, QUESTION_BYTES, QUESTIONS};

/// This run's asks under `parent`.
fn asks(parent: &Path) -> Asks {
    Asks::for_run(parent, "run-1").expect("a new run directory")
}

#[test]
fn a_question_is_written_and_its_answer_read_back() {
    // Given: the lead's answer already there
    let parent = tempfile::tempdir().expect("a scratch directory");
    let asks = asks(parent.path());
    std::fs::write(asks.dir.join("a1.md"), "use the rows page").expect("written");

    // When
    let answer = asks.ask("which page?\n\tthe rows one?", Duration::from_secs(5));

    // Then
    assert_eq!(answer, Ok("use the rows page".to_owned()));
    let asked = std::fs::read_to_string(asks.dir.join("q1.md")).expect("the question");
    assert_eq!(asked, "which page?\n\tthe rows one?");
}

#[test]
fn a_run_directory_already_there_is_refused_so_a_stale_answer_cannot_match() {
    // Given: an earlier run's answer
    let parent = tempfile::tempdir().expect("a scratch directory");
    std::fs::create_dir(parent.path().join("run-1")).expect("made");
    std::fs::write(parent.path().join("run-1/a1.md"), "stale").expect("written");

    // When
    let refused = Asks::for_run(parent.path(), "run-1")
        .err()
        .expect("refused");

    // Then
    assert!(
        refused.contains("each run's questions start in a new directory"),
        "{refused}"
    );
}

#[test]
fn no_answer_by_the_deadline_tells_the_session_to_block_and_stop() {
    let parent = tempfile::tempdir().expect("a scratch directory");
    let refused = asks(parent.path())
        .ask("anyone?", Duration::ZERO)
        .expect_err("no answer");
    assert!(refused.contains("BLOCKED.md"), "{refused}");
}

#[test]
fn an_answer_still_being_written_under_its_tmp_name_is_not_read() {
    // Given: the answer written, not yet renamed
    let parent = tempfile::tempdir().expect("a scratch directory");
    let asks = asks(parent.path());
    std::fs::write(asks.dir.join("a1.md.tmp"), "half an ans").expect("written");

    // When
    let unanswered = asks.ask("first?", Duration::ZERO);
    std::fs::rename(asks.dir.join("a1.md.tmp"), asks.dir.join("a2.md")).expect("renamed");
    let answered = asks.ask("second?", Duration::ZERO);

    // Then
    assert!(unanswered.is_err(), "{unanswered:?}");
    assert_eq!(answered, Ok("half an ans".to_owned()));
}

#[test]
fn an_answer_that_cannot_be_read_or_is_empty_is_refused_naming_it() {
    // Given: a1.md a directory, a2.md empty
    let parent = tempfile::tempdir().expect("a scratch directory");
    let asks = asks(parent.path());
    std::fs::create_dir(asks.dir.join("a1.md")).expect("made");
    std::fs::write(asks.dir.join("a2.md"), " \n").expect("written");

    // When: each with a deadline it would otherwise wait out
    let unreadable = asks
        .ask("one?", Duration::from_secs(600))
        .expect_err("refused");
    let empty = asks
        .ask("two?", Duration::from_secs(600))
        .expect_err("refused");

    // Then
    assert!(
        unreadable.contains("a1.md could not be read"),
        "{unreadable}"
    );
    assert!(empty.contains("a2.md is empty"), "{empty}");
}

#[test]
fn a_question_too_long_or_holding_a_control_byte_is_refused_and_not_written() {
    let parent = tempfile::tempdir().expect("a scratch directory");
    let asks = asks(parent.path());
    let long = "x".repeat(QUESTION_BYTES + 1);
    let too_long = asks.ask(&long, Duration::ZERO).expect_err("refused");
    let control = asks
        .ask("set \x1b]52;c;aGk=\x07", Duration::ZERO)
        .expect_err("refused");
    assert!(too_long.contains("over the 65536"), "{too_long}");
    assert!(control.contains("'\\u{1b}' at byte 4"), "{control}");
    assert_eq!(std::fs::read_dir(&asks.dir).expect("read").count(), 0);
}

#[test]
fn a_question_past_the_runs_cap_is_refused() {
    let parent = tempfile::tempdir().expect("a scratch directory");
    let asks = asks(parent.path());
    for _ in 0..QUESTIONS {
        let _ = asks.ask("again?", Duration::ZERO);
    }
    let refused = asks.ask("once more?", Duration::ZERO).expect_err("refused");
    assert!(
        refused.starts_with("question 21 is past the 20"),
        "{refused}"
    );
    assert!(!asks.dir.join("q21.md").exists());
}
