//! `appearance` (CSS UI 4): `none` takes away what vapors draws of a
//! control's own (a checkbox's mark, a switch's track and knob, a
//! checkbox's natural size) and leaves its behaviour. Chromium 157 is
//! the reference (`docs/appearance.md`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::support::{document, element, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;
use vapors_scene::{Color, Item, Quad, Rect};

const SHEET: &str = "body { margin: 0; } input { display: block; margin: 0; }";

/// A checkbox (a switch when `switch`) with `attributes`, under `style`.
fn page(style: &str, switch: bool, attributes: &[(&str, &str)]) -> (Document, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let input = element(&mut doc, body, "input", "");
    doc.set_attribute(input, "type", "checkbox");
    if switch {
        doc.set_attribute(input, "switch", "");
    }
    for (name, value) in attributes {
        doc.set_attribute(input, name, value);
    }
    doc.add_stylesheet("app.css", &format!("{SHEET} {style}"))
        .expect("parses");
    (doc, input)
}

fn quads(doc: &mut Document) -> Vec<Quad> {
    let painted = doc.update(viewport(400.0, 200.0)).expect("updates");
    let scene = painted.scene().to_scene();
    let all = scene.items().iter();
    all.filter_map(|i| match i {
        Item::Quad(q) => Some(*q),
        _ => None,
    })
    .collect()
}

const NONE_40_BY_22: &str = "input { appearance: none; width: 40px; height: 22px }";

#[test]
fn a_switch_with_appearance_none_draws_no_track_or_knob_in_either_state() {
    for checked in [false, true] {
        // Given
        let attributes: &[(&str, &str)] = if checked { &[("checked", "")] } else { &[] };
        let (mut doc, _) = page(NONE_40_BY_22, true, attributes);

        // When
        let quads = quads(&mut doc);

        // Then
        assert!(quads.is_empty(), "checked {checked}: {quads:?}");
    }
}

#[test]
fn a_switch_with_appearance_none_draws_its_own_background_and_radius() {
    // Given: the author's track
    let style =
        format!("{NONE_40_BY_22} input {{ background-color: #d1d5db; border-radius: 11px }}");
    let (mut doc, _) = page(&style, true, &[]);

    // When
    let quads = quads(&mut doc);

    // Then: one quad, the box's own
    let [track] = &quads[..] else {
        panic!("{quads:?}")
    };
    assert_eq!(track.rect, Rect::new(0.0, 0.0, 40.0, 22.0));
    assert_eq!(track.radii, [11.0; 4]);
    assert_eq!(track.background, Color::from_rgba8(0xd1, 0xd5, 0xdb, 255));
}

#[test]
fn a_checked_checkbox_with_appearance_none_draws_no_mark() {
    // Given: the same checkbox unchecked with its look, and checked without it
    let (mut unchecked, _) = page("input { width: 20px; height: 20px }", false, &[]);
    let style = "input { appearance: none; width: 20px; height: 20px }";
    let (mut checked, _) = page(style, false, &[("checked", "")]);

    // When
    let (checked, unchecked) = (quads(&mut checked), quads(&mut unchecked));

    // Then: the user-agent box alone, no mark
    assert_eq!(checked, unchecked);
}

#[test]
fn a_type_written_in_capitals_keeps_its_parts_as_in_chromium() {
    // Given: Chromium 157 gives `type=Checkbox` `appearance: auto` (docs/appearance.md)
    let (mut plain, _) = page(
        "input { width: 20px; height: 20px }",
        false,
        &[("checked", "")],
    );
    let (mut capitals, input) = page(
        "input { width: 20px; height: 20px }",
        false,
        &[("checked", "")],
    );
    capitals.set_attribute(input, "type", "Checkbox");

    // When
    let (plain, capitals) = (quads(&mut plain), quads(&mut capitals));

    // Then: its mark is drawn in the mark colour, as a lowercase type's is
    // (the user-agent border rule still matches `type` with regard to case,
    // so the box and the mark's place differ, docs/appearance.md)
    let mark = |quads: &[Quad]| quads.last().map(|q| q.background);
    assert_eq!(mark(&capitals), mark(&plain), "{capitals:?}");
}

#[test]
fn an_unsized_checkbox_with_appearance_none_has_no_content_as_in_chromium() {
    // Given: a flex item, as Chromium 157.0.8103.0 lays this out 0 x 0 in a row
    // (getBoundingClientRect, docs/appearance.md); vapors' border is its
    // user-agent look, so it goes
    let style =
        "body { display: flex; align-items: flex-start } input { appearance: none; border: 0 }";
    let (mut doc, input) = page(style, false, &[]);

    // When
    let painted = doc.update(viewport(400.0, 200.0)).expect("updates");

    // Then
    let r = painted.border_box(input).expect("a box");
    assert_eq!((r.width, r.height), (0.0, 0.0));
}

#[test]
fn a_switch_with_appearance_none_still_toggles_on_a_click_and_is_a_switch() {
    // Given
    let (mut doc, input) = page(NONE_40_BY_22, true, &[("aria-label", "Live")]);
    doc.update(viewport(400.0, 200.0)).expect("updates");
    let doc = Rc::new(RefCell::new(doc));

    // When
    Document::pointer_pressed(&doc, [20.0, 11.0]);
    Document::pointer_released(&doc, [20.0, 11.0]);

    // Then
    let mut doc = doc.borrow_mut();
    doc.update(viewport(400.0, 200.0)).expect("updates");
    assert_eq!(doc.checked(input), Some(true));
    let tree = doc.ax_tree().expect("an updated document");
    assert_eq!(
        (tree[0].role.as_str(), tree[0].states.checked),
        ("switch", Some(true))
    );
}

#[test]
fn a_class_that_sets_appearance_none_takes_the_track_and_knob_away_on_the_next_update() {
    // Given: a drawn switch
    let style = "input { width: 40px; height: 22px } .bare { appearance: none }";
    let (mut doc, input) = page(style, true, &[]);
    assert_eq!(quads(&mut doc).len(), 2, "the track and knob before");

    // When
    doc.set_attribute(input, "class", "bare");

    // Then
    assert_eq!(quads(&mut doc), []);
}

#[test]
fn a_class_that_sets_appearance_none_takes_an_unsized_checkboxs_content_away() {
    // Given: an unsized checkbox in a row, 11 x 11 inside a 1 px border
    let style = "body { display: flex; align-items: flex-start } .bare { appearance: none }";
    let (mut doc, input) = page(style, false, &[]);
    let size = |doc: &mut Document| {
        let painted = doc.update(viewport(400.0, 200.0)).expect("updates");
        let r = painted.border_box(input).expect("a box");
        (r.width, r.height)
    };
    assert_eq!(size(&mut doc), (13.0, 13.0), "before");

    // When
    doc.set_attribute(input, "class", "bare");

    // Then: the user-agent border alone
    assert_eq!(size(&mut doc), (2.0, 2.0));
}
