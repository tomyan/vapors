//! Both switches' knobs: a checkbox with `appearance: none` is the track
//! and a decorative sibling the knob, a white 18 px circle 2 px inside with
//! the spec's `SHADOW_SMALL`, at the start off and the end on. A press on
//! the knob reaches the checkbox, which keeps its focus, name and state.
use super::support::*;
use crate::app::Mounted;
use crate::dashboard::*;
use vapors_elements::BoxRect;
use vapors_scene::{Color, Item, Quad, Rect, Shadow};

/// A switch by its class, and its box's top left in the window.
struct Switch {
    class: &'static str,
    x: f32,
    y: f32,
}

const SWITCHES: [Switch; 2] = [
    Switch {
        class: "live-switch",
        x: LIVE_X,
        y: LIVE_Y,
    },
    Switch {
        class: "switch",
        x: SWITCH_X,
        y: SWITCH_Y,
    },
];

const KNOB: f32 = 18.0;
const INSET: f32 = 2.0;

impl Switch {
    fn inside(&self, rect: Rect) -> bool {
        rect.left >= self.x
            && rect.top >= self.y
            && rect.right <= self.x + SWITCH_WIDTH
            && rect.bottom <= self.y + SWITCH_HEIGHT
    }

    /// The knob's box: 2 px inside the track, at its start or end.
    fn knob(&self, on: bool) -> Rect {
        let left = if on {
            self.x + SWITCH_WIDTH - INSET - KNOB
        } else {
            self.x + INSET
        };
        let top = self.y + INSET;
        Rect::new(left, top, left + KNOB, top + KNOB)
    }

    fn centre(&self, on: bool) -> [f32; 2] {
        let knob = self.knob(on);
        [knob.left + KNOB / 2.0, knob.top + KNOB / 2.0]
    }
}

/// The white round quads and the shadows painted inside `switch`'s box.
fn knobs_and_shadows(mounted: &Mounted, switch: &Switch) -> (Vec<Quad>, Vec<Shadow>) {
    let doc = mounted.document.borrow();
    let scene = doc.painted().expect("laid out").scene().to_scene();
    let white = Color::from_rgba8(255, 255, 255, 255);
    let (mut knobs, mut shadows) = (Vec::new(), Vec::new());
    for item in scene.items() {
        match item {
            Item::Quad(q) if switch.inside(q.rect) && q.background == white => knobs.push(*q),
            Item::Shadow(s) if switch.inside(s.rect) => shadows.push(*s),
            _ => {}
        }
    }
    (knobs, shadows)
}

/// `SHADOW_SMALL`'s colour: `SHADOW` at its alpha.
fn small_shadow_colour() -> Color {
    let [_, r, g, b] = SHADOW.to_be_bytes();
    Color {
        a: SHADOW_SMALL.alpha,
        ..Color::from_rgba8(r, g, b, 255)
    }
}

/// The ax nodes whose bounds overlap `switch`'s box: role and checked state.
fn ax_in(mounted: &Mounted, switch: &Switch) -> Vec<(String, Option<bool>)> {
    let doc = mounted.document.borrow();
    let overlaps = |b: &BoxRect| {
        b.x < switch.x + SWITCH_WIDTH
            && b.x + b.width > switch.x
            && b.y < switch.y + SWITCH_HEIGHT
            && b.y + b.height > switch.y
    };
    let mut stack = doc.ax_tree().expect("laid out");
    let mut found = Vec::new();
    while let Some(n) = stack.pop() {
        let small = n.bounds.width <= SWITCH_WIDTH && n.bounds.height <= SWITCH_HEIGHT;
        if small && overlaps(&n.bounds) {
            found.push((n.role.clone(), n.states.checked));
        }
        stack.extend(n.children);
    }
    found
}

#[test]
fn each_switch_draws_one_knob_with_the_small_shadow_in_both_states() {
    for switch in &SWITCHES {
        let mounted = laid_out();
        for on in [false, true] {
            // Given: off, then on after a click
            if on {
                click(&mounted, switch.centre(false));
            }

            // When
            let (knobs, shadows) = knobs_and_shadows(&mounted, switch);

            // Then: one circle, and under it one shadow moved down 1 px
            let state = format!("{} on {on}", switch.class);
            let [knob] = &knobs[..] else {
                panic!("{state}: {knobs:?}")
            };
            assert_eq!(
                (knob.rect, knob.radii),
                (switch.knob(on), [KNOB / 2.0; 4]),
                "{state}"
            );
            let [shadow] = &shadows[..] else {
                panic!("{state}: {shadows:?}")
            };
            let moved = switch.knob(on);
            let y = SHADOW_SMALL.y;
            let moved = Rect::new(moved.left, moved.top + y, moved.right, moved.bottom + y);
            assert_eq!(
                (shadow.rect, shadow.blur),
                (moved, SHADOW_SMALL.blur),
                "{state}"
            );
            let (got, want) = (shadow.color, small_shadow_colour());
            assert!(
                (got.a - want.a).abs() <= 1.0 / 255.0,
                "{state}: {got:?} vs {want:?}"
            );
            assert_eq!((got.r, got.g, got.b), (want.r, want.g, want.b), "{state}");
        }
    }
}

#[test]
fn a_press_on_the_knob_toggles_the_switch_and_focuses_it() {
    for switch in &SWITCHES {
        // Given
        let mounted = laid_out();
        let input = by_class(&mounted.document.borrow(), switch.class)[0];

        // When: pressed on the knob, on and off again
        click(&mounted, switch.centre(false));
        let on = ax_in(&mounted, switch);
        click(&mounted, switch.centre(true));

        // Then
        let focused = mounted.document.borrow().active_element();
        assert_eq!(on, [("switch".to_owned(), Some(true))], "{}", switch.class);
        assert_eq!(focused, Some(input), "{}", switch.class);
        let off = ax_in(&mounted, switch);
        assert_eq!(
            off,
            [("switch".to_owned(), Some(false))],
            "{}",
            switch.class
        );
    }
}
