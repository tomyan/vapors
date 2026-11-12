//! GPU tier: a surface outside the target is culled (docs/gpu-surface.md):
//! inactive as a wholly clipped one is, its backing released and its
//! renderer not called, and active again when it comes back into view.

#[path = "support/surface.rs"]
mod support;

use support::{RED, Rig, Solid, document, update};
use vapors_dom::Dom;

const TRACK: &str = ".track { width: 16px; height: 16px; } .away { margin-left: 40px; }";

#[test]
fn gpu_a_surface_outside_the_target_is_inactive_until_back_in_view() {
    // Given: a red renderer on a 16 x 16 surface in view, drawn once
    let mut doc = document(TRACK, &[(None, "gpu-surface", "track")]);
    update(&mut doc, 1.0);
    let mut rig = Rig::new();
    let (solid, log) = Solid::new(RED);
    rig.painter.mount("track", Box::new(solid)).expect("mounts");
    rig.draw(&doc).expect("draws");
    let surface = doc.children(doc.body())[0];

    // When: moved right of the 32 px target, invalidated, then moved back
    doc.set_attribute(surface, "class", "track away");
    update(&mut doc, 1.0);
    rig.draw(&doc).expect("draws");
    rig.painter.invalidate("track").expect("mounted");
    let wanted_while_away = rig.painter.wants_frame();
    doc.set_attribute(surface, "class", "track");
    update(&mut doc, 1.0);
    rig.draw(&doc).expect("draws");

    // Then: not rendered while away; a new backing and a render on return
    assert!(!wanted_while_away);
    let log = log.borrow();
    assert_eq!((log.mounts, log.resizes.len(), log.frames.len()), (1, 2, 2));
}
