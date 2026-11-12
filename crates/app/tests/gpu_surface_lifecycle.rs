//! GPU tier: when `ScenePainter` calls a surface's renderer
//! (docs/gpu-surface.md): mount once, resize when the backing size
//! changes, render only when invalidated, resized or animating, drop on
//! unmount; a hidden surface is inactive; failures refuse the frame.

#[path = "support/surface.rs"]
mod support;

use support::{BLUE, RED, Rig, Solid, document, pixel, update};
use vapors_app::{MountError, PaintError};
use vapors_dom::Dom;
use vapors_elements::Document;

const TRACK: &str = ".track { width: 16px; height: 16px; } .wide { width: 24px; } \
    .hidden { visibility: hidden; }";

/// A rig with a `Solid` red renderer mounted on a 16 x 16 surface, drawn once.
fn drawn_once(solid: Solid) -> (Rig, Document) {
    let mut doc = document(TRACK, &[(None, "gpu-surface", "track")]);
    update(&mut doc, 1.0);
    let mut rig = Rig::new();
    rig.painter.mount("track", Box::new(solid)).expect("mounts");
    rig.draw(&doc).expect("draws");
    (rig, doc)
}

/// Sets the class of the document's only `gpu-surface`, and updates.
fn restyle(doc: &mut Document, class: &str) {
    let surface = doc.children(doc.body())[0];
    doc.set_attribute(surface, "class", class);
    update(doc, 1.0);
}

#[test]
fn gpu_a_frame_with_nothing_changed_composites_the_last_render() {
    // Given
    let (solid, log) = Solid::new(RED);
    let (mut rig, doc) = drawn_once(solid);

    // When
    let rgba = rig.draw(&doc).expect("draws");

    // Then
    assert_eq!(pixel(&rgba, 4, 4), RED);
    let log = log.borrow();
    assert_eq!((log.mounts, log.resizes.len(), log.frames.len()), (1, 1, 1));
    assert!(!rig.painter.wants_frame());
}

#[test]
fn gpu_invalidating_a_surface_renders_it_on_the_next_frame_only() {
    // Given
    let (solid, log) = Solid::new(RED);
    let (mut rig, doc) = drawn_once(solid);

    // When
    rig.painter.invalidate("track").expect("mounted");
    let wanted = rig.painter.wants_frame();
    rig.draw(&doc).expect("draws");
    rig.draw(&doc).expect("draws");

    // Then
    assert!(wanted);
    assert_eq!(log.borrow().frames.len(), 2);
    assert!(!rig.painter.wants_frame());
}

#[test]
fn gpu_an_animating_surface_renders_every_frame_and_wants_the_next() {
    // Given
    let (solid, log) = Solid::new(RED);
    let (mut rig, doc) = drawn_once(solid);

    // When
    rig.painter.set_animating("track", true).expect("mounted");
    rig.draw(&doc).expect("draws");
    rig.draw(&doc).expect("draws");

    // Then
    assert_eq!(log.borrow().frames.len(), 3);
    assert!(rig.painter.wants_frame());
}

#[test]
fn gpu_resizing_the_surface_renders_a_new_backing_at_the_new_size_once() {
    // Given
    let (mut solid, log) = Solid::new(RED);
    solid.edge = Some(BLUE);
    let (mut rig, mut doc) = drawn_once(solid);

    // When
    restyle(&mut doc, "track wide");
    rig.draw(&doc).expect("draws");
    let rgba = rig.draw(&doc).expect("draws");

    // Then
    let widths =
        |sizes: &[vapors_app::SurfaceSize]| sizes.iter().map(|s| s.width).collect::<Vec<_>>();
    assert_eq!(widths(&log.borrow().resizes), [16, 24]);
    assert_eq!(widths(&log.borrow().frames), [16, 24]);
    assert_eq!([pixel(&rgba, 22, 4), pixel(&rgba, 23, 4)], [RED, BLUE]);
}

#[test]
fn gpu_a_hidden_surface_is_inactive_until_shown_again() {
    // Given
    let (solid, log) = Solid::new(RED);
    let (mut rig, mut doc) = drawn_once(solid);

    // When
    restyle(&mut doc, "track hidden");
    rig.draw(&doc).expect("draws");
    rig.painter.invalidate("track").expect("mounted");
    let wanted_while_hidden = rig.painter.wants_frame();
    restyle(&mut doc, "track");
    rig.draw(&doc).expect("draws");

    // Then
    assert!(!wanted_while_hidden);
    let log = log.borrow();
    assert_eq!((log.mounts, log.resizes.len(), log.frames.len()), (1, 2, 2));
}

#[test]
fn gpu_unmounting_drops_the_renderer_and_the_surface_is_then_refused() {
    // Given
    let (solid, log) = Solid::new(RED);
    let (mut rig, doc) = drawn_once(solid);

    // When
    rig.painter.unmount("track").expect("mounted");
    let refused = rig.draw(&doc).expect_err("refused");

    // Then
    assert!(log.borrow().dropped);
    assert!(matches!(refused, PaintError::Unmounted(ref name) if name == "track"));
}

#[test]
fn gpu_a_renderer_remounted_on_a_name_while_idle_wants_a_frame_and_renders_in_it() {
    // Given
    let (mut rig, doc) = drawn_once(Solid::new(RED).0);
    let (blue, log) = Solid::new(BLUE);

    // When
    rig.painter.unmount("track").expect("mounted");
    rig.painter.mount("track", Box::new(blue)).expect("mounts");
    let wanted = rig.painter.wants_frame();
    let rgba = rig.draw(&doc).expect("draws");

    // Then
    assert!(wanted);
    assert_eq!(pixel(&rgba, 4, 4), BLUE);
    assert_eq!(log.borrow().frames.len(), 1);
    assert!(!rig.painter.wants_frame());
}

#[test]
fn gpu_hidden_and_display_none_surfaces_may_share_a_drawn_surface_name() {
    // Given
    let css = format!("{TRACK} .gone {{ display: none; }}");
    let classes = ["track", "track hidden", "track gone"];
    let mut doc = document(&css, &classes.map(|class| (None, "gpu-surface", class)));
    for surface in doc.children(doc.body()) {
        doc.set_attribute(surface, "name", "track");
    }
    update(&mut doc, 1.0);
    let mut rig = Rig::new();
    rig.painter
        .mount("track", Box::new(Solid::new(RED).0))
        .expect("mounts");

    // When
    let drawn = rig.draw(&doc);

    // Then
    assert!(drawn.is_ok(), "{:?}", drawn.err());
}

#[test]
fn gpu_a_failing_renderer_refuses_the_frame_naming_the_surface() {
    // Given
    let mut doc = document(TRACK, &[(None, "gpu-surface", "track")]);
    update(&mut doc, 1.0);
    let mut rig = Rig::new();
    let (mut solid, _) = Solid::new(RED);
    solid.fails = true;
    rig.painter.mount("track", Box::new(solid)).expect("mounts");

    // When
    let refused = rig.draw(&doc).expect_err("refused");

    // Then
    assert!(matches!(refused, PaintError::Renderer { ref surface, .. } if surface == "track"));
    let message = refused.to_string();
    assert!(
        message.contains("\"track\"") && message.contains("did not compile"),
        "{message}"
    );
}

#[test]
fn gpu_mounting_a_name_twice_and_naming_no_mounted_surface_are_refused() {
    // Given
    let mut rig = Rig::new();
    rig.painter
        .mount("track", Box::new(Solid::new(RED).0))
        .expect("mounts");

    // When
    let twice = rig.painter.mount("track", Box::new(Solid::new(RED).0));
    let unknown = rig.painter.invalidate("trak");
    let unmounted = rig.painter.unmount("trak").map(|_| ());

    // Then
    assert!(matches!(twice, Err(MountError::AlreadyMounted(ref name)) if name == "track"));
    assert!(matches!(unknown, Err(MountError::NotMounted(ref name)) if name == "trak"));
    assert!(matches!(unmounted, Err(MountError::NotMounted(_))));
}
