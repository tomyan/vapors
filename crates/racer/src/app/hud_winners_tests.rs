//! The racer's HUD takes every keyed element's style from its key's
//! compiled winners (build-time-styles.md, B3b).

use super::Racer;
use vapors_elements::Viewport;

#[test]
fn the_hud_takes_its_keyed_styles_from_compiled_winners() {
    // Given
    let racer = Racer::new();
    let view = Viewport {
        width: 1100.0,
        height: 720.0,
        scale: 1.0,
    };

    // When
    racer.sync_hud(view);
    let updated = racer.document.borrow_mut().update(view).map(drop);
    updated.unwrap_or_else(|errors| panic!("racer HUD refused: {errors:?}"));

    // Then
    let counts = racer.document.borrow().update_counts();
    assert_eq!(counts.winners_runtime, 0, "{counts:?}");
    assert!(counts.winners_compiled > 0, "{counts:?}");
}
