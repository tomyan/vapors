//! Pointer capture across a `gpu-surface`'s edge through
//! `ElementsApp::give` (docs/gpu-surface.md, Input): a press on a
//! surface that captures keeps going to it past its edge, with offsets
//! beyond its box, until the release; a press on the HUD that captures
//! stays with the HUD over a surface. A touch is captured by what it
//! went down on without asking.

mod surface_input_page;

use surface_input_page::{PANELS, Page};
use vapors_platform::TouchPhase;

#[test]
fn a_captured_drag_from_the_left_surface_keeps_reaching_it_past_its_edge() {
    // Given: pressed on the left surface, which captures
    let mut page = Page::new(PANELS, true, 1.0);
    page.move_to(180.0, 100.0);
    page.take_log();
    page.button(true);

    // When: over the right surface, above the panels, released on the right
    page.move_to(250.0, 100.0);
    page.move_to(5.0, 5.0);
    page.move_to(250.0, 190.0);
    page.button(false);

    // Then: offsets from (10, 30), negative and past the 180 x 140 box;
    // the click is the capturing element's, as in Chromium
    let expected = [
        "left pointerdown 170,70",
        "left pointermove 240,70",
        "left pointermove -5,-25",
        "left pointermove 240,160",
        "left pointerup 240,160",
        "left click 240,160",
    ];
    assert_eq!(page.take_log(), expected);
}

#[test]
fn after_the_captured_release_moves_go_to_the_surface_under_the_pointer() {
    // Given: a captured drag from the left surface released over the right
    let mut page = Page::new(PANELS, true, 1.0);
    page.move_to(180.0, 100.0);
    page.button(true);
    page.move_to(250.0, 100.0);
    page.button(false);
    page.take_log();

    // When
    page.move_to(260.0, 100.0);

    // Then
    assert_eq!(page.take_log(), ["right pointermove 50,70"]);
}

#[test]
fn a_captured_drag_from_the_hud_stays_with_the_hud_over_the_surface() {
    // Given: pressed on the HUD over the left surface, which captures
    let mut page = Page::new(PANELS, true, 1.0);
    page.move_to(70.0, 65.0);
    page.take_log();
    page.button(true);

    // When: across the left surface and released on it
    page.move_to(150.0, 120.0);
    page.button(false);

    // Then: the HUD's offsets from its padding box at (41, 51), inside
    // its 1 px border; the click is the capturing element's, as in Chromium
    let expected = [
        "hud pointerdown 29,14",
        "hud pointermove 109,69",
        "hud pointerup 109,69",
        "hud click 109,69",
    ];
    assert_eq!(page.take_log(), expected);
}

#[test]
fn an_uncaptured_drag_from_the_hud_moves_and_releases_on_the_surface_without_its_press() {
    // Given: no listener captures
    let mut page = Page::new(PANELS, false, 1.0);
    page.move_to(70.0, 65.0);
    page.take_log();
    page.button(true);

    // When
    page.move_to(150.0, 120.0);
    page.button(false);

    // Then: as on the web, the surface gets the move and the release, no
    // pointerdown or click
    let expected = [
        "hud pointerdown 29,14",
        "left pointermove 140,90",
        "left pointerup 140,90",
    ];
    assert_eq!(page.take_log(), expected);
}

#[test]
fn a_touch_on_the_left_surface_is_captured_without_asking() {
    // Given: no listener captures
    let mut page = Page::new(PANELS, false, 1.0);
    page.touch(TouchPhase::Started, 180.0, 100.0);

    // When: onto the right surface and lifted there
    page.touch(TouchPhase::Moved, 250.0, 100.0);
    page.touch(TouchPhase::Ended, 250.0, 100.0);

    // Then: every event to the left surface, its offsets past the edge
    let expected = [
        "left pointerdown 170,70",
        "left pointermove 240,70",
        "left pointerup 240,70",
    ];
    assert_eq!(page.take_log(), expected);
}
