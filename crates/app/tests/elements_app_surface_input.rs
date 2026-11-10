//! Pointer input on a `gpu-surface` through `ElementsApp::give`
//! (docs/gpu-surface.md, Input): a surface's events carry `offset` from
//! its content box's top-left in CSS pixels, at any device scale and
//! scroll; UI drawn over a surface takes the press; each surface gets
//! only its own events.

mod surface_input_page;

use surface_input_page::{PANELS, Page, scrolling};
use vapors_app::Route;

/// A click at (`x`, `y`) in CSS pixels: the press's route and the log.
fn click(page: &mut Page, x: f32, y: f32) -> (Route, Vec<String>) {
    page.move_to(x, y);
    page.take_log();
    let route = page.button(true);
    page.button(false);
    (route, page.take_log())
}

#[test]
fn a_click_on_the_left_surface_carries_its_surface_local_offset() {
    // Given
    let mut page = Page::new(PANELS, false, 1.0);

    // When
    let (route, log) = click(&mut page, 150.0, 120.0);

    // Then: (150, 120) less the content box's origin (10, 30)
    assert_eq!(route, Route::Document);
    let at = "140,90";
    let expected = [
        format!("left pointerdown {at}"),
        format!("left pointerup {at}"),
        format!("left click {at}"),
    ];
    assert_eq!(log, expected);
}

#[test]
fn at_device_scale_two_the_offset_is_in_css_pixels() {
    // Given: physical positions are twice the CSS ones
    let mut page = Page::new(PANELS, false, 2.0);

    // When
    let (_, log) = click(&mut page, 150.0, 120.0);

    // Then
    assert_eq!(
        log.first().map(String::as_str),
        Some("left pointerdown 140,90")
    );
}

#[test]
fn the_right_surface_gets_its_own_offset_and_the_left_gets_nothing() {
    // Given
    let mut page = Page::new(PANELS, false, 1.0);

    // When: (300, 100) less the right content box's origin (210, 30)
    let (_, log) = click(&mut page, 300.0, 100.0);

    // Then
    assert_eq!(
        log,
        [
            "right pointerdown 90,70",
            "right pointerup 90,70",
            "right click 90,70"
        ]
    );
}

#[test]
fn a_surface_scrolled_in_its_panel_counts_the_offset_from_where_it_now_is() {
    // Given: the left panel scrolled down 40 px; its surface starts at y -10
    let mut page = Page::new(&scrolling(), false, 1.0);
    let scrolled = page
        .doc
        .borrow_mut()
        .scroll_to(page.left_panel, [0.0, 40.0]);
    assert!(scrolled, "the left panel did not scroll");
    page.update();

    // When
    let (_, log) = click(&mut page, 100.0, 100.0);

    // Then
    assert_eq!(
        log.first().map(String::as_str),
        Some("left pointerdown 90,110")
    );
}

#[test]
fn a_press_on_the_hud_over_the_surface_is_the_huds_and_the_surface_gets_nothing() {
    // Given
    let mut page = Page::new(PANELS, false, 1.0);

    // When: (70, 65) is inside the HUD button at (40, 50), over the left
    // surface; its padding box starts inside a 1 px border
    let (route, log) = click(&mut page, 70.0, 65.0);

    // Then
    assert_eq!(route, Route::Document);
    assert_eq!(
        log,
        [
            "hud pointerdown 29,14",
            "hud pointerup 29,14",
            "hud click 29,14"
        ]
    );
}

#[test]
fn hovering_moves_reach_the_surface_under_the_pointer_and_stop_over_the_hud() {
    // Given
    let mut page = Page::new(PANELS, false, 1.0);

    // When: over the left surface, onto the HUD, off the panels, out of the window
    page.move_to(150.0, 120.0);
    page.move_to(70.0, 65.0);
    page.move_to(200.0, 180.0);
    page.give(&vapors_platform::Event::PointerLeft);

    // Then: one move each; no event says the pointer left either
    assert_eq!(
        page.take_log(),
        ["left pointermove 140,90", "hud pointermove 29,14"]
    );
}
