//! A switch (`input type=checkbox switch`): painted geometry, `accent-color`,
//! toggling, events, the tree. The sizes are the brief's (`docs/switch.md`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::support::{document, element, viewport};
use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::Document;
use vapors_scene::{Color, Item, Quad, Rect};

const SHEET: &str = "body { margin: 0; } input { display: block; margin: 0; }";

fn page(style: &str, attributes: &[(&str, &str)]) -> (Document, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let input = element(&mut doc, body, "input", "");
    doc.set_attribute(input, "type", "checkbox");
    doc.set_attribute(input, "switch", "");
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
    all.filter_map(|i| {
        if let Item::Quad(q) = i {
            Some(*q)
        } else {
            None
        }
    })
    .collect()
}

#[test]
fn a_switch_is_2_25_em_by_1_25_em_so_it_scales_with_font_size() {
    for (font, size) in [(16.0, (36.0, 20.0)), (40.0, (90.0, 50.0))] {
        // Given
        let (mut doc, input) = page(&format!("input {{ font-size: {font}px }}"), &[]);

        // When
        let painted = doc.update(viewport(400.0, 200.0)).expect("updates");

        // Then
        let r = painted.border_box(input).expect("a box");
        assert_eq!((r.width, r.height), size, "at {font}px");
    }
}

#[test]
fn unchecked_the_knob_sits_at_the_start_on_a_grey_track() {
    // Given: 44 px font: a 99 x 55 box, the knob inset 5 and 45 across
    let (mut doc, _) = page("input { font-size: 44px }", &[]);

    // When
    let quads = quads(&mut doc);

    // Then
    let [track, knob] = &quads[..] else {
        panic!("{quads:?}")
    };
    assert_eq!(track.rect, Rect::new(0.0, 0.0, 99.0, 55.0));
    assert_eq!(track.radii, [27.5; 4]);
    assert_eq!(track.background, Color::from_rgba8(208, 208, 215, 255));
    assert_eq!(knob.rect, Rect::new(5.0, 5.0, 50.0, 50.0));
    assert_eq!(knob.radii, [22.5; 4]);
    assert_eq!(knob.background, Color::from_rgba8(255, 255, 255, 255));
}

#[test]
fn checked_the_knob_moves_to_the_end_and_the_track_takes_the_accent() {
    // Given: 88 px font, the accent colour on the body, which inherits
    let style = "input { font-size: 88px } body { accent-color: #4f46e5 }";
    let (mut doc, _) = page(style, &[("checked", "")]);

    // When
    let quads = quads(&mut doc);

    // Then: 198 x 110, inset 10, knob 90
    let [track, knob] = &quads[..] else {
        panic!("{quads:?}")
    };
    assert_eq!(track.rect, Rect::new(0.0, 0.0, 198.0, 110.0));
    assert_eq!(track.background, Color::from_rgba8(0x4f, 0x46, 0xe5, 255));
    assert_eq!(knob.rect, Rect::new(98.0, 10.0, 188.0, 100.0));
}

#[test]
fn the_ua_blue_for_auto_and_the_track_colour_part_when_unchecked() {
    // Given / When
    let on = quads(&mut page("", &[("checked", "")]).0);
    let off = quads(&mut page("input { --vapors-track-color: #d1d5db }", &[]).0);

    // Then
    assert_eq!(on[0].background, Color::from_rgba8(0, 96, 223, 255));
    assert_eq!(off[0].background, Color::from_rgba8(0xd1, 0xd5, 0xdb, 255));
}

fn click(doc: &Shared<Document>, at: [f32; 2]) {
    Document::pointer_pressed(doc, at);
    Document::pointer_released(doc, at);
}

type Shared<T> = Rc<RefCell<T>>;

fn logged(doc: Document, node: NodeId) -> (Shared<Document>, Shared<Vec<String>>) {
    let doc = Rc::new(RefCell::new(doc));
    let log = Rc::new(RefCell::new(Vec::new()));
    for name in ["input", "change"] {
        let into = Rc::clone(&log);
        doc.borrow_mut().listen(
            node,
            name,
            Box::new(move |e: &mut Event| {
                into.borrow_mut()
                    .push(format!("{} {:?}", e.name(), e.checked()));
            }),
        );
    }
    (doc, log)
}

#[test]
fn a_click_toggles_and_fires_input_then_change_and_checked_selects() {
    // Given: a switch whose :checked rule moves it
    let (mut doc, input) = page("input:checked { margin-left: 50px }", &[]);
    doc.update(viewport(400.0, 200.0)).expect("updates");
    let (doc, log) = logged(doc, input);

    // When
    click(&doc, [5.0, 5.0]);
    let moved_x = {
        let mut doc = doc.borrow_mut();
        let painted = doc.update(viewport(400.0, 200.0)).expect("updates");
        painted.border_box(input).expect("a box").x
    };

    // Then
    assert_eq!(doc.borrow().checked(input), Some(true));
    assert_eq!(*log.borrow(), ["input Some(true)", "change Some(true)"]);
    assert_eq!(moved_x, 50.0);
}

#[test]
fn space_toggles_a_focused_switch_and_enter_does_not() {
    // Given
    let (mut doc, input) = page("", &[]);
    doc.update(viewport(400.0, 200.0)).expect("updates");
    let (doc, log) = logged(doc, input);
    let none = vapors_elements::Modifiers::default();
    Document::focus(&doc, Some(input));

    // When
    Document::key_pressed(&doc, "Enter", none);
    Document::key_released(&doc, "Enter", none);
    let after_enter = doc.borrow().checked(input);
    Document::key_pressed(&doc, " ", none);
    Document::key_released(&doc, " ", none);

    // Then
    assert_eq!(after_enter, Some(false));
    assert_eq!(doc.borrow().checked(input), Some(true));
    assert_eq!(*log.borrow(), ["input Some(true)", "change Some(true)"]);
}

#[test]
fn a_disabled_switch_takes_no_click() {
    // Given
    let (mut doc, input) = page("", &[("disabled", "")]);
    doc.update(viewport(400.0, 200.0)).expect("updates");
    let (doc, log) = logged(doc, input);

    // When
    click(&doc, [5.0, 5.0]);

    // Then
    assert_eq!(doc.borrow().checked(input), Some(false));
    assert!(log.borrow().is_empty());
}

#[test]
fn the_tree_names_a_switch_with_its_checked_state() {
    // Given
    let (mut doc, _) = page("", &[("aria-label", "Compare"), ("checked", "")]);
    doc.update(viewport(400.0, 200.0)).expect("updates");

    // When
    let tree = doc.ax_tree().expect("an updated document");

    // Then
    assert_eq!(tree[0].role, "switch");
    assert_eq!(tree[0].states.checked, Some(true));
}
