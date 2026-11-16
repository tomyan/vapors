//! A demo page takes every keyed element's style from its key's
//! compiled winners (build-time-styles.md, B3b): no element of a
//! generated key falls back to the full cascade.

use vapors_elements::Viewport;

/// The Save button (toasts.rs's tests).
const SAVE: [u32; 2] = [530, 21];
const VIEWPORT: Viewport = Viewport {
    width: 640.0,
    height: 400.0,
    scale: 1.0,
};

#[test]
fn the_toasts_demo_takes_its_keyed_styles_from_compiled_winners() {
    // Given
    let demo = vapors_demos::find("toasts").expect("a demo");
    let document = vapors_demos::document(&demo).document;

    let update = || {
        let updated = document.borrow_mut().update(VIEWPORT).map(drop);
        updated.unwrap_or_else(|errors| panic!("toasts was refused: {errors:#?}"));
    };
    update();

    // When Save raises a toast, so the list's row keys (`.toast`,
    // `.message`, `.dismiss`) are restyled too
    vapors_demos::click(&document, SAVE);
    update();

    // Then
    let counts = document.borrow().update_counts();
    assert_eq!(counts.winners_runtime, 0, "{counts:?}");
    assert!(counts.winners_compiled >= 3, "{counts:?}");
}
