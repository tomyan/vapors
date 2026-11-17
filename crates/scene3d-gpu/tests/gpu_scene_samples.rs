//! GPU tier: a tilted cube's edges at 1 and 4 samples against a
//! reference drawn at 8 times the size and averaged down, 64 samples a
//! pixel (`docs/plans/flat-scene-f7.md`, plan item 5): 4 samples come
//! nearer the reference at the edges.

mod support;

use support::{Fixture, SIZE, render_sized};
use vapors_app::SurfaceSize;
use vapors_renderer::Samples;
use vapors_scene3d::{GeometryHandle, Projection, Quat, Transform, Vec3};

/// The reference's size, in backing pixels a pixel each way.
const SCALE: u32 = 8;

/// A face: its normal, its across and up directions, and its shade.
type Face = ([f32; 3], [f32; 3], [f32; 3], f32);

/// A cube 2 wide, centred on the origin, each face's four corners its own
/// with its own shade, so the faces part.
fn cube(fixture: &Fixture) -> GeometryHandle {
    let faces: [Face; 6] = [
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], 1.0),
        ([0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], 0.9),
        ([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0], 0.75),
        ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0], 0.6),
        ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0], 0.5),
        ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], 0.4),
    ];
    let (mut positions, mut indices, mut shades) = (Vec::new(), Vec::new(), Vec::new());
    for (normal, across, up, shade) in faces {
        shades.extend([shade; 4]);
        let first = u32::try_from(positions.len()).expect("24 corners");
        let [n, a, u] = [normal, across, up].map(Vec3::from);
        positions.extend([n - a - u, n + a - u, n + a + u, n - a + u]);
        indices.extend([0, 1, 2, 0, 2, 3].map(|at| first + at));
    }
    fixture
        .meshes
        .borrow_mut()
        .geometry(positions, indices, shades)
        .unwrap()
}

/// A fixture of the cube turned to show three faces, its edges off the
/// pixel grid.
fn tilted_cube() -> Fixture {
    let fixture = Fixture::new();
    fixture.camera(
        Projection::perspective(0.9, 0.1, 100.0).unwrap(),
        Vec3::new(0.0, 0.0, 6.0),
    );
    let turned =
        Quat::from_rotation_x(0.47) * Quat::from_rotation_y(0.71) * Quat::from_rotation_z(0.13);
    let local = Transform::new(Vec3::ZERO, turned, Vec3::ONE).unwrap();
    let geometry = cube(&fixture);
    fixture.mesh(fixture.root(), &geometry, [200, 120, 60, 255], local);
    fixture
}

/// The cube drawn at `samples` into a backing `scale` times `SIZE`.
fn drawn(samples: Samples, scale: u32) -> Vec<u8> {
    let gpu = vapors_gpu_harness::gpu();
    let size = SurfaceSize {
        width: SIZE.width * scale,
        height: SIZE.height * scale,
        scale: 1.0,
    };
    let fixture = tilted_cube();
    let mut renderer = fixture.renderer_at(&gpu, (size, samples));
    render_sized(&mut renderer, &gpu, size).expect("renders").0
}

/// `big`, `SCALE` times `SIZE`, averaged down to `SIZE`, each channel
/// the mean of its `SCALE * SCALE` (64) pixels.
fn averaged_down(big: &[u8]) -> Vec<f32> {
    let (width, height) = (SIZE.width as usize, SIZE.height as usize);
    let (scale, wide) = (SCALE as usize, (SIZE.width * SCALE) as usize);
    let mut out = vec![0.0; width * height * 4];
    for (at, value) in out.iter_mut().enumerate() {
        let (pixel, channel) = (at / 4, at % 4);
        let (x, y) = (pixel % width, pixel / width);
        let sum: u32 = (0..scale * scale)
            .map(|k| (x * scale + k % scale, y * scale + k / scale))
            .map(|(bx, by)| u32::from(big[(by * wide + bx) * 4 + channel]))
            .sum();
        // at most 64 * 255: exact in f32
        *value = f32::from(u16::try_from(sum).expect("64 pixels of a byte")) / 64.0;
    }
    out
}

/// Over the reference's edge pixels (alpha neither 0 nor 255, or a
/// colour between two faces'), the mean and the largest channel
/// difference of `drawn` from `reference`, and how many pixels.
fn edge_error(drawn: &[u8], reference: &[f32]) -> (f32, f32, usize) {
    let edges: Vec<usize> = (0..reference.len() / 4)
        .filter(|&p| reference[p * 4 + 3] > 0.5 && reference[p * 4 + 3] < 254.5)
        .collect();
    let differences: Vec<f32> = edges
        .iter()
        .flat_map(|&p| (0..4).map(move |c| p * 4 + c))
        .map(|at| (f32::from(drawn[at]) - reference[at]).abs())
        .collect();
    #[allow(clippy::cast_precision_loss)] // a few thousand differences
    let mean = differences.iter().sum::<f32>() / differences.len() as f32;
    let most = differences.iter().copied().fold(0.0, f32::max);
    (mean, most, edges.len())
}

#[test]
fn gpu_a_tilted_cubes_edges_at_4_samples_are_nearer_the_reference_than_at_1() {
    // Given
    let reference = averaged_down(&drawn(Samples::One, SCALE));

    // When
    let one = edge_error(&drawn(Samples::One, 1), &reference);
    let four = edge_error(&drawn(Samples::Four, 1), &reference);

    // Then: measured 2026-11-17 on ANV (UHD 630) and llvmpipe, over the
    // same 231 edge pixels: mean 29.06 (largest 127.5) at 1 sample, 10.05
    // (60.3) at 4, on both; 4 samples must stay under half 1's
    assert_eq!((one.2, four.2), (231, 231), "the edge pixels changed");
    assert!(four.0 < one.0 / 2.0, "1x {one:?}, 4x {four:?}");
    assert!(four.1 < one.1, "1x {one:?}, 4x {four:?}");
}
