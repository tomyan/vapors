//! What the sidebar does: hover, click, the window and the refusals,
//! through vapors' test drivers.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)] // WINDOW is whole
use super::support::*;
use crate::app::{Mounted, not_built, options, run};
use crate::cli::Scenario;
use crate::dashboard::*;
use vapors_dom::NodeId;
use vapors_elements::Document;

/// A colour as the style engine writes it.
fn css(color: u32) -> String {
    let [_, r, g, b] = color.to_be_bytes();
    format!("rgb({r} {g} {b})")
}

fn paint(doc: &Document, node: NodeId, property: &str) -> String {
    let style = doc.computed_style(node).expect("styled");
    style.get_by_name(property).expect("a property").to_css()
}

fn centre(doc: &Document, node: NodeId) -> [f32; 2] {
    let [x, y, w, h] = rect(doc, node);
    [x + w / 2.0, y + h / 2.0]
}

/// Moves the pointer onto the centre of the `nth` element of `class`, then lays out.
fn hover(mounted: &Mounted, class: &str, nth: usize) {
    let point = {
        let doc = mounted.document.borrow();
        centre(&doc, by_class(&doc, class)[nth])
    };
    Document::pointer_moved(&mounted.document, point);
    refresh(mounted);
}

fn item_background(mounted: &Mounted, nth: usize) -> String {
    let doc = mounted.document.borrow();
    paint(&doc, by_class(&doc, "item")[nth], "background-color")
}

fn icon_colours(mounted: &Mounted) -> Vec<String> {
    let doc = mounted.document.borrow();
    let icons = by_class(&doc, "icon");
    icons
        .into_iter()
        .map(|icon| paint(&doc, icon, "background-color"))
        .collect()
}

#[test]
fn the_first_nav_item_is_active_at_startup() {
    // Given
    let mounted = laid_out();

    // Then
    let icons = icon_colours(&mounted);
    assert_eq!(icons[0], css(NAV_ICON_ACTIVE));
    assert!(icons[1..].iter().all(|icon| *icon == css(NAV_ICON_COLOR)));
    assert_eq!(item_background(&mounted, 0), css(NAV_ACTIVE));
}

#[test]
fn the_pointer_over_a_nav_item_that_is_not_active_gives_it_the_hover_colour() {
    // Given
    let mounted = laid_out();
    let before = item_background(&mounted, 3);

    // When
    hover(&mounted, "item", 3);

    // Then
    assert_ne!(before, css(NAV_HOVER));
    assert_eq!(item_background(&mounted, 3), css(NAV_HOVER));
    assert_eq!(item_background(&mounted, 4), before);
}

#[test]
fn the_pointer_over_the_active_item_keeps_its_active_colour() {
    // Given
    let mounted = laid_out();

    // When
    hover(&mounted, "item", 0);

    // Then
    assert_eq!(item_background(&mounted, 0), css(NAV_ACTIVE));
}

#[test]
fn a_click_makes_the_item_active_and_the_old_one_plain() {
    // Given
    let mounted = laid_out();
    let point = {
        let doc = mounted.document.borrow();
        centre(&doc, by_class(&doc, "item")[2])
    };

    // When
    Document::pointer_moved(&mounted.document, point);
    Document::pointer_pressed(&mounted.document, point);
    Document::pointer_released(&mounted.document, point);
    refresh(&mounted);

    // Then
    assert_eq!(mounted.active.get_untracked(), 2);
    assert_eq!(item_background(&mounted, 2), css(NAV_ACTIVE));
    assert_ne!(item_background(&mounted, 0), css(NAV_ACTIVE));
    let icons = icon_colours(&mounted);
    assert_eq!(icons[2], css(NAV_ICON_ACTIVE));
    assert_eq!(icons[0], css(NAV_ICON_COLOR));
}

#[test]
fn the_window_is_borderless_1280_by_800_with_the_runners_app_id() {
    // When
    let window = options("gui-bench app");

    // Then
    assert_eq!((window.width as f32, window.height as f32), WINDOW);
    assert!(!window.decorations);
    assert_eq!(window.app_id.as_deref(), Some("gui-bench.vapors"));
    assert_eq!(window.title, Scenario::App { count: 100 }.title());
}

#[test]
fn the_list_scenario_is_refused_naming_s3b() {
    // When
    let list = run(Scenario::List { count: 10 }).unwrap_err();

    // Then
    assert_eq!(list, not_built("the `list` scenario"));
    assert!(list.contains("s3b") && list.contains("vapors-bench --scenario app"));
}
