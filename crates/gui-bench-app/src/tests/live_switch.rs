//! The live switch's painted track and knob in each state, after a click:
//! the checkbox's background (`appearance: none`) and its sibling knob.
//! Its keyboard, name and checked state are in `switch_keys.rs`, its
//! knob's shadow in `switch_knob.rs`; the corpus fixture `live_switch`
//! holds the same to Chromium.
use super::support::*;
use crate::app::Mounted;
use crate::dashboard::*;
use vapors_scene::{Item, Quad};

const CENTRE: [f32; 2] = [LIVE_X + 20.0, LIVE_Y + 11.0];

fn checked(mounted: &Mounted) -> bool {
    mounted.controls.live.on.get()
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
fn a_click_toggles_it_and_paints_the_track_and_knob_of_each_state() {
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
