use super::*;

#[test]
fn the_first_refusal_is_kept() {
    // Given
    let counts = Counts::new(Arc::new(AtomicU64::new(0)));

    // When
    counts.refuse("first");
    counts.refuse("second");

    // Then
    let outcome = counts.outcome().expect_err("refused");
    assert!(outcome.starts_with("first\n"), "{outcome}");
    assert!(!outcome.contains("second"), "{outcome}");
}

#[test]
fn a_refusal_carries_what_the_app_did_each_frame() {
    // Given
    let counts = Counts::new(Arc::new(AtomicU64::new(0)));
    counts
        .trace
        .lock()
        .expect("not poisoned")
        .note(3, "the app acted on RequestAnimation".into());

    // When
    counts.refuse("animating failed");

    // Then
    let outcome = counts.outcome().expect_err("refused");
    assert!(outcome.starts_with("animating failed\n"), "{outcome}");
    assert!(
        outcome.contains("after frame 3: the app acted on RequestAnimation"),
        "{outcome}"
    );
}
