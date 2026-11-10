use super::{DEADLINE_FRAMES, Scopes};

#[test]
fn a_scope_unresolved_for_the_deadline_refuses_every_later_frame() {
    // Given
    let mut scopes = Scopes::default();
    scopes.wait_for(Box::pin(std::future::pending()));

    // When
    let before: Vec<bool> = (1..DEADLINE_FRAMES)
        .map(|_| scopes.check().is_ok())
        .collect();
    let at = scopes.check().map_err(|refusal| format!("{refusal:?}"));
    let after = scopes.check();

    // Then
    assert!(before.iter().all(|ok| *ok));
    assert!(
        at.as_ref().is_err_and(|e| e.contains("within 10 frames")),
        "{at:?}"
    );
    assert!(after.is_err());
    assert!(!scopes.waiting());
}
