//! Both switches' knobs: a checkbox with `appearance: none` is the track
//! and a decorative sibling the knob, a white 18 px circle 2 px inside with
//! the spec's `SHADOW_SMALL`, at the start off and the end on. A press on
//! the knob reaches the checkbox, which takes focus and keeps its name.
use super::support::*;
use super::switches::{KNOB, SWITCHES, Switch};
use crate::app::Mounted;
use crate::dashboard::*;
use vapors_scene::{Color, Item, Quad, Rect, Shadow};

/// The white round quads and the shadows painted inside `switch`'s box.
fn knobs_and_shadows(mounted: &Mounted, switch: &Switch) -> (Vec<Quad>, Vec<Shadow>) {
    let doc = mounted.document.borrow();
    let scene = doc.painted().expect("laid out").scene().to_scene();
    let white = rgb(0x00ff_ffff);
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
    Color {
        a: SHADOW_SMALL.alpha,
        ..rgb(SHADOW)
    }
}

/// `rect` moved by `SHADOW_SMALL`'s offset.
fn moved(rect: Rect) -> Rect {
    let (x, y) = (SHADOW_SMALL.x, SHADOW_SMALL.y);
    Rect::new(rect.left + x, rect.top + y, rect.right + x, rect.bottom + y)
}

/// Asserts the one shadow is `SHADOW_SMALL` under `knob`.
fn assert_small_shadow(shadows: &[Shadow], knob: Rect, state: &str) {
    let [shadow] = shadows else {
        panic!("{state}: {shadows:?}")
    };
    let geometry = (shadow.rect, shadow.radii, shadow.blur);
    let want = (moved(knob), [KNOB / 2.0; 4], SHADOW_SMALL.blur);
    assert_eq!(geometry, want, "{state}");
    let (got, want) = (shadow.color, small_shadow_colour());
    assert!(
        (got.a - want.a).abs() <= 1.0 / 255.0,
        "{state}: {got:?} vs {want:?}"
    );
    assert_eq!((got.r, got.g, got.b), (want.r, want.g, want.b), "{state}");
}

#[test]
fn each_switch_draws_one_knob_with_the_small_shadow_in_both_states() {
    for switch in &SWITCHES {
        for on in [false, true] {
            // Given
            let mounted = laid_out();
            switch.set(&mounted, on);
            refresh(&mounted);

            // When
            let (knobs, shadows) = knobs_and_shadows(&mounted, switch);

            // Then: one circle, and under it one shadow
            let state = format!("{} on {on}", switch.class);
            let [knob] = &knobs[..] else {
                panic!("{state}: {knobs:?}")
            };
            let want = (switch.knob(on), [KNOB / 2.0; 4]);
            assert_eq!((knob.rect, knob.radii), want, "{state}");
            assert_small_shadow(&shadows, switch.knob(on), &state);
        }
    }
}

#[test]
fn a_press_on_the_knob_toggles_the_switch_and_focuses_it() {
    for switch in &SWITCHES {
        // Given
        let mounted = laid_out();

        // When: pressed on the knob, on and off again
        click(&mounted, switch.centre(false));
        let on = switch.ax(&mounted);
        click(&mounted, switch.centre(true));

        // Then
        let focused = mounted.document.borrow().active_element();
        assert_eq!(on, switch.expected_ax(true), "{}", switch.class);
        assert_eq!(focused, Some(switch.input(&mounted)), "{}", switch.class);
        assert_eq!(
            switch.ax(&mounted),
            switch.expected_ax(false),
            "{}",
            switch.class
        );
    }
}
