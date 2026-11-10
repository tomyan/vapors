//! Root module so the test ids start with `gpu_` (the tier split).

use vapors_platform::Instant;

use crate::painter::{SCOPE_DEADLINE, wants_frame_around_a_pending_scope};

#[test]
fn gpu_an_idle_backed_surface_with_a_pending_scope_wants_no_frame_until_its_deadline() {
    // Given
    let now = Instant::now();

    // When
    let [before, after] = wants_frame_around_a_pending_scope(now);

    // Then
    assert_eq!(before, (false, None));
    assert_eq!(after, (false, Some(now + SCOPE_DEADLINE)));
}
