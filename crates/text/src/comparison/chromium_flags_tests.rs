//! The render flags, as passed and as recorded.

use super::*;

#[test]
fn the_render_flags_name_the_window_and_scale() {
    // When
    let flags = render_flags(640, 400, 2.0);

    // Then
    assert_eq!(
        flags,
        [
            "--headless=new",
            "--no-sandbox",
            "--hide-scrollbars",
            "--force-device-scale-factor=2",
            "--virtual-time-budget=5000",
            "--window-size=640,400",
        ]
    );
}
