//! The racer's HUD takes every keyed element's style from its key's
//! compiled winners (build-time-styles.md, B3b). The world beside it
//! (`ui/racer_world.wisp`, R3a2) is not the HUD: its five anchors with
//! no class (the scene, the camera, two groups and the ground) take the
//! scene components' own keys, which no compiler analyses, so they are
//! styled by the full cascade at run time, as every scene anchor is.

use super::Racer;
use vapors_elements::Viewport;

/// The world's anchors styled at run time: those with no class. No rule
/// of the world's sheet can match them, so the cascade only computes
/// defaults, and a racing frame restyles nothing (`gpu_hud_iu5_tests.rs`).
/// Retired when the compiler analyses scene components' keys
/// (one-scene.md section 7, Open items), when this becomes 0.
const WORLD_OWN_KEYED: usize = 5;

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
    assert_eq!(counts.winners_runtime, WORLD_OWN_KEYED, "{counts:?}");
    assert!(counts.winners_compiled > 0, "{counts:?}");
}
