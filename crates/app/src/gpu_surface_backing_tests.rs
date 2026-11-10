//! Root module so the test ids start with `gpu_` (the tier split).

use crate::painter::wants_frame_around_a_pending_scope;

#[test]
fn gpu_an_idle_backed_surface_with_a_pending_scope_wants_a_frame() {
    // Given / When
    let (before, after) = wants_frame_around_a_pending_scope();

    // Then
    assert!(!before);
    assert!(after);
}
