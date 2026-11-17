use std::time::Duration;

use super::Asks;

#[test]
fn a_question_is_written_and_its_answer_read_back() {
    // Given: the lead's answer already there
    let dir = tempfile::tempdir().expect("a scratch directory");
    std::fs::write(dir.path().join("a1.md"), "use the rows page").expect("written");
    let asks = Asks::new(dir.path().to_owned());

    // When
    let answer = asks.ask("which page?", Duration::from_secs(5));

    // Then
    assert_eq!(answer, Ok("use the rows page".to_owned()));
    let asked = std::fs::read_to_string(dir.path().join("q1.md")).expect("the question");
    assert_eq!(asked, "which page?");
}

#[test]
fn no_answer_by_the_deadline_tells_the_session_to_block_and_stop() {
    // Given
    let dir = tempfile::tempdir().expect("a scratch directory");
    let asks = Asks::new(dir.path().to_owned());

    // When
    let refused = asks.ask("anyone?", Duration::ZERO).expect_err("no answer");

    // Then
    assert!(refused.contains("BLOCKED.md"), "{refused}");
}

#[test]
fn each_question_takes_the_next_number() {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let asks = Asks::new(dir.path().to_owned());
    let _ = asks.ask("first", Duration::ZERO);
    let _ = asks.ask("second", Duration::ZERO);
    assert!(dir.path().join("q2.md").exists());
}
