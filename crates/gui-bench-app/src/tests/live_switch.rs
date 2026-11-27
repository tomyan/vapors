//! The live switch is the framework's `switch` part: its pointer, keyboard,
//! name, checked state and painted geometry, as a browser checkbox has them
//! (`tests/live_switch_oracle.rs` holds the same to Chromium).
use super::support::*;
use crate::app::Mounted;
use crate::dashboard::*;
use vapors_elements::{Document, Modifiers};
use vapors_scene::{Item, Quad};

const CENTRE: [f32; 2] = [LIVE_X + 20.0, LIVE_Y + 11.0];

fn checked(mounted: &Mounted) -> bool {
    mounted.controls.live.on.get()
}

/// The ax node of the live switch: its role, name and checked state.
fn ax(mounted: &Mounted) -> (String, String, Option<bool>) {
    let doc = mounted.document.borrow();
    let tree = doc.ax_tree().expect("laid out");
    let mut stack: Vec<_> = tree.iter().collect();
    while let Some(n) = stack.pop() {
        if n.role == "switch" && n.bounds.x == LIVE_X {
            return (n.role.clone(), n.name.clone(), n.states.checked);
        }
        stack.extend(n.children.iter());
    }
    panic!(
        "no switch at x {LIVE_X} in the ax tree (cargo test -p vapors-gui-bench-app live_switch)"
    );
}

/// The quads inside the switch's box: the track, then the knob.
fn quads(mounted: &Mounted) -> Vec<Quad> {
    let doc = mounted.document.borrow();
    let scene = doc.painted().expect("laid out").scene().to_scene();
    let inside = |q: &Quad| {
        q.rect.left >= LIVE_X
            && q.rect.top >= LIVE_Y
            && q.rect.right <= LIVE_X + SWITCH_WIDTH
            && q.rect.bottom <= LIVE_Y + SWITCH_HEIGHT
    };
    let items = scene.items().iter();
    items
        .filter_map(|i| match i {
            Item::Quad(q) if inside(q) => Some(*q),
            _ => None,
        })
        .collect()
}

#[test]
fn a_click_toggles_it_and_the_part_paints_the_track_and_knob_of_each_state() {
    // Given: off, a grey track with the knob at the start
    let mounted = laid_out();
    let off = quads(&mounted);
    assert!(!checked(&mounted));
    assert_eq!(
        (off[0].background, off[1].rect.left),
        (rgb(SWITCH_OFF), LIVE_X + 2.0)
    );

    // When
    click(&mounted, CENTRE);

    // Then: the accent track, the knob at the end
    let on = quads(&mounted);
    assert!(checked(&mounted));
    assert_eq!(
        (on[0].background, on[1].rect.left),
        (rgb(PRIMARY), LIVE_X + SWITCH_WIDTH - 20.0)
    );
}

fn rgb(color: u32) -> vapors_scene::Color {
    let [_, r, g, b] = color.to_be_bytes();
    vapors_scene::Color::from_rgba8(r, g, b, 255)
}

#[test]
fn space_toggles_the_focused_switch_and_enter_does_not() {
    // Given: focused by a click (then off again by Space)
    let mounted = laid_out();
    click(&mounted, CENTRE);
    let doc = &mounted.document;

    // When / Then
    Document::key_pressed(doc, "Enter", Modifiers::default());
    Document::key_released(doc, "Enter", Modifiers::default());
    assert!(checked(&mounted), "Enter does nothing to a checkbox");
    Document::key_pressed(doc, " ", Modifiers::default());
    Document::key_released(doc, " ", Modifiers::default());
    assert!(!checked(&mounted), "Space toggles on keyup");
}

#[test]
fn it_is_a_switch_named_live_updates_with_its_checked_state() {
    // Given, When, Then
    let mounted = laid_out();
    assert_eq!(
        ax(&mounted),
        ("switch".into(), LIVE_LABEL.into(), Some(false))
    );
    click(&mounted, CENTRE);
    assert_eq!(
        ax(&mounted),
        ("switch".into(), LIVE_LABEL.into(), Some(true))
    );
}
