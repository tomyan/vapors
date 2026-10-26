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
