//! GPU tier: how `ScenePainter` composes a `gpu-surface`'s renderer
//! (docs/gpu-surface.md): over the content box in device pixels, under
//! an ancestor's rectangular clip, beneath UI painted after it, one
//! renderer per surface; a surface with no renderer is refused by name.

#[path = "support/surface.rs"]
mod support;

use support::{BLUE, RED, Rig, SIZE, Solid, WHITE, document, pixel, update};
use vapors_app::{PaintError, SurfaceSize};

const AT_8: &str = ".track { position: absolute; left: 8px; top: 8px; width: 16px; height: 16px; }";

#[test]
fn gpu_a_mounted_surface_shows_its_renderer_over_its_content_box() {
    // Given
    let mut doc = document(AT_8, &[(None, "gpu-surface", "track")]);
    update(&mut doc, 1.0);
    let mut rig = Rig::new();
    let (solid, log) = Solid::new(RED);
    rig.painter.mount("track", Box::new(solid)).expect("mounts");

    // When
    let rgba = rig.draw(&doc).expect("draws");

    // Then
    assert_eq!([pixel(&rgba, 8, 8), pixel(&rgba, 23, 23)], [RED, RED]);
    assert_eq!([pixel(&rgba, 7, 7), pixel(&rgba, 24, 24)], [WHITE, WHITE]);
    let size = SurfaceSize {
        width: 16,
        height: 16,
        scale: 1.0,
    };
    assert_eq!(log.borrow().resizes, [size]);
}

#[test]
fn gpu_at_scale_two_the_backing_is_in_device_pixels() {
    // Given
    let css = ".track { position: absolute; left: 4px; top: 4px; width: 8px; height: 8px; }";
    let mut doc = document(css, &[(None, "gpu-surface", "track")]);
    update(&mut doc, 2.0);
    let mut rig = Rig::new();
    let (solid, log) = Solid::new(RED);
    rig.painter.mount("track", Box::new(solid)).expect("mounts");

    // When
    let rgba = rig.draw(&doc).expect("draws");

    // Then
    assert_eq!([pixel(&rgba, 8, 8), pixel(&rgba, 23, 23)], [RED, RED]);
    assert_eq!(pixel(&rgba, 24, 24), WHITE);
    let size = SurfaceSize {
        width: 16,
        height: 16,
        scale: 2.0,
    };
    assert_eq!(log.borrow().resizes, [size]);
    assert_eq!(size.logical(), [8.0, 8.0]);
}

#[test]
fn gpu_an_overflow_hidden_ancestor_clips_the_surface() {
    // Given
    let css = ".panel { position: absolute; left: 8px; top: 8px; width: 8px; height: 8px; \
        overflow: hidden; } .track { width: 16px; height: 16px; }";
    let elements = [(None, "div", "panel"), (Some(0), "gpu-surface", "track")];
    let mut doc = document(css, &elements);
    update(&mut doc, 1.0);
    let mut rig = Rig::new();
    let solid = Box::new(Solid::new(RED).0);
    rig.painter.mount("track", solid).expect("mounts");

    // When
    let rgba = rig.draw(&doc).expect("draws");

    // Then
    assert_eq!([pixel(&rgba, 8, 8), pixel(&rgba, 15, 15)], [RED, RED]);
    assert_eq!([pixel(&rgba, 16, 16), pixel(&rgba, 20, 20)], [WHITE, WHITE]);
}

#[test]
fn gpu_ui_painted_after_the_surface_is_drawn_over_it() {
    // Given
    let css = format!(
        "{AT_8} .hud {{ position: absolute; left: 12px; top: 12px; width: 4px; height: 4px; \
         background-color: #0000ff; }}"
    );
    let elements = [(None, "gpu-surface", "track"), (None, "div", "hud")];
    let mut doc = document(&css, &elements);
    update(&mut doc, 1.0);
    let mut rig = Rig::new();
    let solid = Box::new(Solid::new(RED).0);
    rig.painter.mount("track", solid).expect("mounts");

    // When
    let rgba = rig.draw(&doc).expect("draws");

    // Then
    assert_eq!([pixel(&rgba, 12, 12), pixel(&rgba, 15, 15)], [BLUE, BLUE]);
    assert_eq!([pixel(&rgba, 9, 9), pixel(&rgba, 16, 16)], [RED, RED]);
}

#[test]
fn gpu_two_panels_show_their_own_renderers() {
    // Given
    let half = SIZE / 2;
    let css = format!(
        "gpu-surface {{ position: absolute; width: {half}px; height: {half}px; }} \
         .left {{ left: 0; top: 0; }} .right {{ left: {half}px; top: {half}px; }}"
    );
    let elements = [
        (None, "gpu-surface", "left"),
        (None, "gpu-surface", "right"),
    ];
    let mut doc = document(&css, &elements);
    update(&mut doc, 1.0);
    let mut rig = Rig::new();
    let (left, right) = (Solid::new(RED).0, Solid::new(BLUE).0);
    rig.painter.mount("left", Box::new(left)).expect("mounts");
    rig.painter.mount("right", Box::new(right)).expect("mounts");

    // When
    let rgba = rig.draw(&doc).expect("draws");

    // Then
    let (last, edge) = (SIZE - 1, half - 1);
    assert_eq!([pixel(&rgba, 0, 0), pixel(&rgba, edge, edge)], [RED, RED]);
    assert_eq!(
        [pixel(&rgba, half, half), pixel(&rgba, last, last)],
        [BLUE, BLUE]
    );
    assert_eq!(
        [pixel(&rgba, half, 0), pixel(&rgba, 0, half)],
        [WHITE, WHITE]
    );
}

#[test]
fn gpu_a_surface_with_no_renderer_refuses_the_frame_naming_it() {
    // Given
    let mut doc = document(AT_8, &[(None, "gpu-surface", "track")]);
    update(&mut doc, 1.0);
    let mut rig = Rig::new();

    // When
    let refused = rig.draw(&doc).expect_err("refused");

    // Then
    assert!(matches!(refused, PaintError::Unmounted(ref name) if name == "track"));
    assert!(refused.to_string().contains("mount(\"track\""), "{refused}");
}
