//! GPU tier: the closed-form pipeline draws rects and ellipses under an
//! affine transform with coverage taken from the distance to the true
//! outline in device pixels, the ramp straddling it: 0.5 on the
//! outline, half a pixel of ramp outside it as well as inside (Tom,
//! 2026-11-13: as Chromium draws an edge). Drawn white over black, a
//! pixel's red channel is its coverage.

use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{
    ClosedForm, ClosedFormPipeline, ClosedFormShape, FrameEncoder, Target, clear,
};
use vapors_scene::{Color, Size};

const WIDTH: u32 = 32;
const HEIGHT: u32 = 32;

/// `form` filled white under `transform`.
fn white(form: ClosedForm, transform: [f32; 6]) -> ClosedFormShape {
    ClosedFormShape {
        form,
        stroke: None,
        transform,
        colour: Color::from_rgba8(255, 255, 255, 255),
    }
}

fn scaled(scale: f32) -> [f32; 6] {
    [scale, 0.0, 0.0, scale, 0.0, 0.0]
}

/// Each pixel's coverage, 0 to 1, row by row.
fn coverage(shapes: &[ClosedFormShape]) -> Vec<f32> {
    let gpu = gpu();
    let target = clear(
        &gpu.device,
        &gpu.queue,
        WIDTH,
        HEIGHT,
        Color::from_rgba8(0, 0, 0, 255),
    );
    let mut pipeline = ClosedFormPipeline::new(&gpu.device, target.format());
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let encoder = FrameEncoder::new(&gpu.device);
    pipeline.draw(
        &Target {
            device: &gpu.device,
            queue: &gpu.queue,
            encoder: &encoder,
            view: &view,
            viewport: Size {
                width: WIDTH,
                height: HEIGHT,
            },
            scissor: None,
        },
        shapes,
    );
    encoder.submit(&gpu.queue).expect("no draw refused");
    let pixels = read_rgba8(&gpu.device, &gpu.queue, &target);
    pixels.chunks(4).map(|p| f32::from(p[0]) / 255.0).collect()
}

fn at(coverage: &[f32], x: u32, y: u32) -> f32 {
    coverage[(y * WIDTH + x) as usize]
}

/// Within one 8-bit step and a little of the rasteriser's rounding.
fn assert_near(found: f32, expected: f32, what: &str) {
    assert!(
        (found - expected).abs() <= 1.5 / 255.0,
        "{what}: coverage {found}, expected {expected}"
    );
}

/// The edges of a rect from x = 10.75 to 20.25 and y = 1.5 to 7, in
/// device pixels, as each scale draws them.
fn assert_rect_edges_straddle(coverage: &[f32]) {
    // Column 9's centre is 1.25 outside the left edge: past the ramp.
    assert_near(at(coverage, 9, 4), 0.0, "column 9");
    // Column 10's centre (10.5) is 0.25 outside: a quarter covered, so
    // the shape has fragments outside its outline.
    assert_near(at(coverage, 10, 4), 0.25, "column 10, outside the edge");
    assert_near(at(coverage, 11, 4), 1.0, "column 11");
    assert_near(at(coverage, 20, 4), 0.25, "column 20, outside the edge");
    // Row 1's centre (1.5) is on the top edge: exactly half.
    assert_near(at(coverage, 15, 1), 0.5, "row 1, on the edge");
    assert_near(at(coverage, 15, 0), 0.0, "row 0");
}

#[test]
fn gpu_a_rect_at_1x_is_half_covered_on_its_outline_and_ramps_outside_it() {
    // When
    let coverage = coverage(&[white(
        ClosedForm::Rect {
            origin: [10.75, 1.5],
            size: [9.5, 5.5],
            radius: 0.0,
        },
        scaled(1.0),
    )]);

    // Then
    assert_rect_edges_straddle(&coverage);
}

#[test]
fn gpu_a_rect_at_2x_ramps_over_one_device_pixel_not_one_unit() {
    // Given: the same rect in units half a device pixel each
    let coverage = coverage(&[white(
        ClosedForm::Rect {
            origin: [5.375, 0.75],
            size: [4.75, 2.75],
            radius: 0.0,
        },
        scaled(2.0),
    )]);

    // Then
    assert_rect_edges_straddle(&coverage);
}

#[test]
fn gpu_a_circle_is_covered_by_its_distance_in_device_pixels() {
    // Given: a circle of radius 6.25 about (12, 12.5) at 1x, and the same
    // at 2x
    for (scale, centre, radius) in [(1.0, [12.0, 12.5], 6.25), (2.0, [6.0, 6.25], 3.125)] {
        let coverage = coverage(&[white(
            ClosedForm::Ellipse {
                centre,
                radii: [radius, radius],
            },
            scaled(scale),
        )]);

        // Then: on row 12 (centre y 12.5), column 17's centre is 0.75
        // inside, column 18's 0.25 outside, column 19's 1.25 outside
        let what = |column: u32| format!("{scale}x, column {column}");
        assert_near(at(&coverage, 17, 12), 1.0, &what(17));
        assert_near(at(&coverage, 18, 12), 0.25, &what(18));
        assert_near(at(&coverage, 19, 12), 0.0, &what(19));
        assert_near(at(&coverage, 12, 12), 1.0, &what(12));
    }
}

#[test]
fn gpu_a_stroke_covers_half_its_width_either_side_of_the_outline() {
    // Given: a rect from x = 8 to 24 stroked 2 wide: covered from 7 to 9
    // on the left side
    let coverage = coverage(&[ClosedFormShape {
        stroke: Some(2.0),
        ..white(
            ClosedForm::Rect {
                origin: [8.0, 8.0],
                size: [16.0, 16.0],
                radius: 0.0,
            },
            scaled(1.0),
        )
    }]);

    // Then
    assert_near(at(&coverage, 6, 16), 0.0, "left of the stroke");
    assert_near(at(&coverage, 7, 16), 1.0, "the stroke's outer half");
    assert_near(at(&coverage, 8, 16), 1.0, "the stroke's inner half");
    assert_near(at(&coverage, 9, 16), 0.0, "inside the stroke");
    assert_near(at(&coverage, 16, 16), 0.0, "the middle");
}

#[test]
fn gpu_a_rotated_rect_covers_its_area() {
    // Given: a 12 x 8 rect rotated 30 degrees about the target's centre;
    // a ramp inside the outline only would lose half a pixel along all
    // 40 pixels of it, 20 of 96
    let (sin, cos) = 30f32.to_radians().sin_cos();
    let transform = [cos, sin, -sin, cos, 16.0, 16.0];
    let coverage = coverage(&[white(
        ClosedForm::Rect {
            origin: [-6.0, -4.0],
            size: [12.0, 8.0],
            radius: 0.0,
        },
        transform,
    )]);

    // Then
    let area: f32 = coverage.iter().sum();
    assert!((area - 96.0).abs() < 1.0, "covered {area} of 96");
    assert_near(at(&coverage, 16, 16), 1.0, "the centre");
}

#[test]
fn gpu_a_later_shape_draws_over_an_earlier_one() {
    // Given: a rect, then a circle over its right half drawn black
    let rect = white(
        ClosedForm::Rect {
            origin: [4.0, 4.0],
            size: [16.0, 8.0],
            radius: 2.0,
        },
        scaled(1.0),
    );
    let circle = ClosedFormShape {
        colour: Color::from_rgba8(0, 0, 0, 255),
        ..white(
            ClosedForm::Ellipse {
                centre: [20.0, 8.0],
                radii: [4.0, 4.0],
            },
            scaled(1.0),
        )
    };

    // When
    let coverage = coverage(&[rect, circle]);

    // Then
    assert_near(at(&coverage, 8, 8), 1.0, "the rect");
    assert_near(at(&coverage, 19, 8), 0.0, "the circle over it");
}
