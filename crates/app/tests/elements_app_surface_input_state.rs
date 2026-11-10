//! What a `gpu-surface` takes from the document besides events
//! (docs/gpu-surface.md, Input): `:hover` applies to it, `pointer-events:
//! none` lets a press through to what is under it, and it takes focus
//! only with a `tabindex`.

mod surface_input_page;

use surface_input_page::{PANELS, Page};
use vapors_dom::Dom;

const HOVER_STYLE: &str = " .left .view { background-color: #111111; } \
    .left .view:hover { background-color: #eeeeee; }";

fn left_background(page: &Page) -> String {
    let doc = page.doc.borrow();
    let style = doc.computed_style(page.left_surface).expect("styled");
    format!("{:?}", style.get_by_name("background-color"))
}

#[test]
fn hover_applies_to_a_surface_under_the_pointer_and_not_under_the_hud() {
    // Given
    let mut page = Page::new(&format!("{PANELS}{HOVER_STYLE}"), false, 1.0);
    let at_rest = left_background(&page);

    // When: over the left surface, then onto the HUD above it, then back
    page.move_to(150.0, 120.0);
    let over_surface = left_background(&page);
    page.move_to(70.0, 65.0);
    let over_hud = left_background(&page);
    page.move_to(300.0, 100.0);
    let over_right = left_background(&page);

    // Then
    assert_ne!(over_surface, at_rest);
    assert_eq!(over_hud, at_rest);
    assert_eq!(over_right, at_rest);
}

#[test]
fn a_surface_with_pointer_events_none_passes_the_press_to_its_panel() {
    // Given
    let style = format!("{PANELS} .right .view {{ pointer-events: none; }}");
    let mut page = Page::new(&style, false, 1.0);
    page.move_to(300.0, 100.0);
    page.take_log();

    // When: (300, 100) is on the right surface; its panel starts at (210, 10)
    page.button(true);
    page.button(false);

    // Then: the panel under it takes the press and the surface none
    let at = "90,90";
    let expected = [
        format!("right-panel pointerdown {at}"),
        format!("right-panel pointerup {at}"),
        format!("right-panel click {at}"),
    ];
    assert_eq!(page.take_log(), expected);
}

#[test]
fn pointer_events_none_on_the_body_reaches_a_surface_but_not_a_button_with_auto() {
    // Given: as the racer's hud.wisp has it, `pointer-events: none` on
    // html and body and `auto` on its buttons
    let style =
        format!("{PANELS} html, body {{ pointer-events: none; }} .hud {{ pointer-events: auto; }}");
    let mut page = Page::new(&style, false, 1.0);

    // When: a click on the left surface, then one on the HUD over it
    page.move_to(150.0, 120.0);
    page.take_log();
    page.button(true);
    page.button(false);
    let on_surface = page.take_log();
    page.move_to(70.0, 65.0);
    page.take_log();
    page.button(true);
    page.button(false);
    let on_hud = page.take_log();

    // Then: the surface inherits none and gets nothing; the button takes both
    assert_eq!(on_surface, Vec::<String>::new());
    assert_eq!(
        on_hud.first().map(String::as_str),
        Some("hud pointerdown 29,14")
    );
}

#[test]
fn a_press_on_a_surface_without_a_tabindex_focuses_nothing() {
    // Given
    let mut page = Page::new(PANELS, false, 1.0);
    page.move_to(150.0, 120.0);

    // When
    page.button(true);
    page.button(false);

    // Then
    assert_eq!(page.doc.borrow().active_element(), None);
}

#[test]
fn a_surface_with_a_tabindex_takes_focus_from_a_press() {
    // Given
    let mut page = Page::new(PANELS, false, 1.0);
    page.doc
        .borrow_mut()
        .set_attribute(page.left_surface, "tabindex", "0");
    page.update();
    page.move_to(150.0, 120.0);

    // When
    page.button(true);
    page.button(false);

    // Then
    assert_eq!(page.doc.borrow().active_element(), Some(page.left_surface));
}

#[test]
fn tab_skips_surfaces_without_a_tabindex_and_stops_at_one_with() {
    // Given: only the right surface has a tabindex
    let mut page = Page::new(PANELS, false, 1.0);
    page.doc
        .borrow_mut()
        .set_attribute(page.right_surface, "tabindex", "0");
    page.update();

    // When: Tab from nothing, then again
    page.key("Tab", true);
    page.key("Tab", false);
    let first = page.doc.borrow().active_element();
    page.key("Tab", true);
    page.key("Tab", false);
    let second = page.doc.borrow().active_element();

    // Then: the right surface, not the left, then the button after it
    assert_eq!(first, Some(page.right_surface));
    assert_eq!(second, Some(page.hud));
}
