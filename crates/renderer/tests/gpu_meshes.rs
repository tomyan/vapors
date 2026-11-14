//! GPU tier: the mesh pipeline draws tessellated fills and strokes with
//! coverage from the nearest outline edge's distance in device pixels,
//! the ramp straddling the outline as the closed forms' does: 0.5 on
//! it, half a pixel of ramp outside, reached by extruding the outline.

#[path = "support/meshes.rs"]
mod support;

use support::{assert_at, coverage, mesh, scaled, white};
use vapors_scene::Color;

/// The edges of a rect from x = 10.75 to 20.25 and y = 1.5 to 7, as
/// each scale draws them (the closed forms' test, as a mesh).
fn assert_rect_edges_straddle(coverage: &[f32]) {
    assert_at(coverage, (9, 4), 0.0, "1.25 outside the left edge");
    assert_at(
        coverage,
        (10, 4),
        0.25,
        "0.25 outside: extruded past the outline",
    );
    assert_at(coverage, (11, 4), 1.0, "inside");
    assert_at(coverage, (20, 4), 0.25, "0.25 outside the right edge");
    assert_at(coverage, (15, 1), 0.5, "on the top edge");
    assert_at(coverage, (15, 0), 0.0, "above it");
}

#[test]
fn gpu_a_filled_mesh_at_1x_is_half_covered_on_its_outline_and_ramps_outside_it() {
    // When
    let coverage = coverage(&[white(mesh("M10.75 1.5h9.5v5.5h-9.5z", None), scaled(1.0))]);

    // Then
    assert_rect_edges_straddle(&coverage);
}

#[test]
fn gpu_a_filled_mesh_at_2x_ramps_over_one_device_pixel() {
    // When
    let coverage = coverage(&[white(
        mesh("M5.375 0.75h4.75v2.75h-4.75z", None),
        scaled(2.0),
    )]);

    // Then
    assert_rect_edges_straddle(&coverage);
}

#[test]
fn gpu_a_slanted_edge_is_covered_by_its_perpendicular_distance() {
    // Given: a triangle whose long side runs along x + y = 24
    let coverage = coverage(&[white(mesh("M4 4L20 4L4 20z", None), scaled(1.0))]);

    // Then
    assert_at(&coverage, (11, 12), 0.5, "a pixel centre on the long side");
    assert_at(&coverage, (12, 12), 0.0, "0.71 outside it");
    assert_at(&coverage, (11, 11), 1.0, "0.71 inside it");
}

#[test]
fn gpu_a_mesh_scaled_three_times_wider_extrudes_far_enough_across_its_short_axis() {
    // Given: x from 11.25 to 17.25, y from 2.1 to 8.1 in device pixels
    let wide = mesh("M3.75 2.1h2v6h-2z", None);
    let coverage = coverage(&[white(wide, [3.0, 0.0, 0.0, 1.0, 0.0, 0.0])]);

    // Then
    assert_at(&coverage, (11, 5), 0.75, "0.25 inside the left edge");
    assert_at(&coverage, (17, 5), 0.25, "0.25 outside the right edge");
    assert_at(&coverage, (14, 2), 0.9, "0.4 inside the top edge");
    assert_at(&coverage, (14, 8), 0.1, "0.4 outside the bottom edge");
}

#[test]
fn gpu_a_stroke_mesh_covers_half_its_width_either_side_of_its_path() {
    // Given: a line along y = 16 stroked 2 wide, and one 2.5 wide
    let two = coverage(&[white(mesh("M4 16h24", Some(2.0)), scaled(1.0))]);
    let wider = coverage(&[white(mesh("M4 16h24", Some(2.5)), scaled(1.0))]);

    // Then
    assert_at(&two, (16, 14), 0.0, "above the stroke");
    assert_at(&two, (16, 15), 1.0, "its upper half");
    assert_at(&two, (16, 16), 1.0, "its lower half");
    assert_at(&two, (16, 17), 0.0, "below it");
    assert_at(
        &wider,
        (16, 14),
        0.25,
        "0.25 outside the wider stroke's edge",
    );
}

#[test]
fn gpu_a_rotated_mesh_covers_its_area_and_a_later_one_draws_over() {
    // Given: a 12 x 8 rect rotated 30 degrees about the centre, then a
    // black square over its middle
    let (sin, cos) = 30f32.to_radians().sin_cos();
    let turned = white(
        mesh("M-6 -4h12v8h-12z", None),
        [cos, sin, -sin, cos, 16.0, 16.0],
    );
    let black = (
        mesh("M15 15h2v2h-2z", None),
        scaled(1.0),
        Color::from_rgba8(0, 0, 0, 255),
    );

    // When
    let alone = coverage(std::slice::from_ref(&turned));
    let covered = coverage(&[turned, black]);

    // Then
    let area: f32 = alone.iter().sum();
    assert!((area - 96.0).abs() < 1.0, "covered {area} of 96");
    assert_at(&covered, (15, 15), 0.0, "the later square over it");
    assert_at(&covered, (13, 16), 1.0, "the rect beside it");
}

#[test]
fn gpu_an_entrys_vertices_are_written_once_and_counted() {
    // Given
    let gpu = vapors_gpu_harness::gpu();
    let triangles = mesh("M0 0h4v4h-4z", None);
    let (target, encoder) = (
        vapors_renderer::clear(
            &gpu.device,
            &gpu.queue,
            4,
            4,
            Color::from_rgba8(0, 0, 0, 255),
        ),
        vapors_renderer::FrameEncoder::new(&gpu.device),
    );
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let on = vapors_renderer::Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder: &encoder,
        view: &view,
        viewport: vapors_scene::Size {
            width: 4,
            height: 4,
        },
        scissor: None,
    };
    let mut entries = vapors_renderer::MeshEntries::new(&gpu.device);

    // When
    let _entry = entries.write(&on, &triangles).expect("room");

    // Then: 40 bytes a vertex, three a triangle, padded to 16
    let expected = (triangles.len() as u64 * 120).next_multiple_of(16);
    assert_eq!(entries.take_work().buffer_bytes, expected);
    assert_eq!(entries.bytes(), expected);
    encoder.submit(&gpu.queue).expect("no refusal");
}
