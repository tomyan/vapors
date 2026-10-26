use super::*;

#[test]
fn a_backed_off_head_is_skipped_until_its_time() {
    let mut state = State::default();
    state.back_off("aaa", 1_000);
    assert!(state.backed_off("aaa", 1_000 + BACKOFF_SECS - 1));
    assert!(!state.backed_off("aaa", 1_000 + BACKOFF_SECS));
    assert!(!state.backed_off("bbb", 1_000));
}

#[test]
fn a_reason_is_new_only_when_it_changed() {
    let mut state = State::default();
    assert!(state.changed(68, "#68 has no review clear"));
    assert!(!state.changed(68, "#68 has no review clear"));
    assert!(state.changed(68, "#68's gate failed at aaa"));
}

#[test]
fn the_state_reads_back_from_its_file() {
    let path = std::env::temp_dir().join(format!("xtask-queue-state-{}.json", std::process::id()));
    let mut state = State::default();
    state.back_off("aaa", 5);
    state.changed(68, "said");
    state.save(&path).expect("saved");
    let read = State::load(&path).expect("read");
    let _ = std::fs::remove_file(&path);
    assert_eq!(read, state);
    assert_eq!(
        State::load(std::path::Path::new("/nonexistent/state.json")),
        Ok(State::default())
    );
}

#[test]
fn a_state_file_of_another_shape_is_refused_naming_it() {
    // Given: valid JSON whose backoff holds a string, not a time
    let path =
        std::env::temp_dir().join(format!("xtask-queue-state-bad-{}.json", std::process::id()));
    std::fs::write(&path, r#"{"backoff":{"aaa":"soon"},"said":{}}"#).expect("written");

    // When
    let read = State::load(&path);
    let _ = std::fs::remove_file(&path);

    // Then
    let why = read.expect_err("refused");
    assert!(why.contains(&path.display().to_string()), "{why}");
    assert!(
        why.contains("not the queue's state (backoff); fix: delete it"),
        "{why}"
    );
}

#[test]
fn an_expired_backoff_is_pruned() {
    // Given
    let mut state = State::default();
    state.back_off("old", 1_000);
    state.back_off("new", 5_000);

    // When
    state.prune(1_000 + BACKOFF_SECS);

    // Then
    assert!(!state.backoff.contains_key("old"));
    assert!(state.backed_off("new", 5_000));
}
