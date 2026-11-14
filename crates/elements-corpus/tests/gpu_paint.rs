//! GPU tier: each paint fixture (a `/* paint */` comment in its style),
//! drawn by vapors offscreen and by headless Chromium at 1x and 2x, is
//! held to the rules in `paint_check`: flat regions within 2, every
//! pixel within 96, and in each element's region at most 2 pixels off
//! by more than half the local contrast. The composite (vapors | Chromium | difference
//! x4) is written to `target/tmp/paint_<name>_<scale>x.png`.
//! The rules are checked in `gpu_paint_rules`. Four pixels of touche's
//! Mute icon at 1x (two in each of the two opaque rows) are compared with exact coverage instead
//! (`paint_support/exact.rs`, declared there).

#[path = "paint_support/exact.rs"]
mod exact;
mod paint_support;

use std::path::PathBuf;

use paint_support::{HEIGHT, WIDTH, chromiums, image, laid_out, paint_fixtures, regions};
use vapors_app::ScenePainter;
use vapors_elements::Document;
use vapors_elements_corpus::paint_check::compare;
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{FrameEncoder, Target, clear};
use vapors_scene::{Color, Size};
use vapors_text::comparison::{Rgba, write_png};

fn ours(doc: &Document, scale: u32) -> Rgba {
    let gpu = gpu();
    let size = Size {
        width: WIDTH * scale,
        height: HEIGHT * scale,
    };
    let texture = clear(
        &gpu.device,
        &gpu.queue,
        size.width,
        size.height,
        Color::default(),
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let encoder = FrameEncoder::new(&gpu.device);
    let target = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        encoder: &encoder,
        view: &view,
        viewport: size,
        scissor: None,
    };
    let painted = doc.painted().expect("painted");
    ScenePainter::new(&gpu.device, texture.format())
        .expect("Rgba8Unorm is drawn")
        .draw(&target, painted)
        .expect("paints");
    encoder.submit(&gpu.queue).expect("no draw refused");
    Rgba {
        width: size.width,
        height: size.height,
        data: read_rgba8(&gpu.device, &gpu.queue, &texture),
    }
}

/// vapors | Chromium | difference x4, for eyes.
fn composite(ours: &Rgba, theirs: &Rgba) -> Rgba {
    let mut data = Vec::with_capacity(ours.data.len() * 3);
    let row = |image: &Rgba, y: u32| {
        image.data[(y * image.width * 4) as usize..((y + 1) * image.width * 4) as usize].to_vec()
    };
    for y in 0..ours.height {
        data.extend(row(ours, y));
        data.extend(row(theirs, y));
        for (p, q) in row(ours, y).chunks(4).zip(row(theirs, y).chunks(4)) {
            let d = (0..3)
                .map(|i| p[i].abs_diff(q[i]))
                .max()
                .unwrap_or(0)
                .saturating_mul(4);
            data.extend([d, d, d, 255]);
        }
    }
    Rgba {
        width: ours.width * 3,
        height: ours.height,
        data,
    }
}

fn check(name: &str, scale: u32) {
    let fixture = paint_fixtures()
        .into_iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| {
            panic!("{name} is not a paint fixture: remove its test in gpu_paint.rs")
        });
    // Given
    let theirs = exact::with_exact_pixels(name, scale, chromiums("match", name, scale, ""));
    let doc = laid_out(&fixture, scale);

    // When
    let ours = ours(&doc.borrow(), scale);

    // Then
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("paint_{name}_{scale}x.png"));
    write_png(&out, &composite(&ours, &theirs)).expect("composite");
    let found = compare(
        &image(&ours),
        &image(&theirs),
        &regions(&doc.borrow(), scale),
    );
    assert!(
        found.is_empty(),
        "{name} at {scale}x ({}) differs from Chromium's: {}",
        out.display(),
        found[..found.len().min(6)].join("; ")
    );
}

/// One test per fixture and scale, so nextest runs them in parallel and
/// each stays inside the 60 s deadline of `.config/nextest.toml`
/// (30 s period, terminated after 2). The `gpu_` prefix keeps them in the
/// gpu tier, which selects `test(/^gpu_/)`. `gpu_every_paint_fixture_has_a_test`
/// holds this list to the fixtures marked `/* paint`.
macro_rules! paint_tests {
    ($($name:ident),* $(,)?) => {
        const TESTED: &[&str] = &[$(stringify!($name)),*];
        mod gpu_at_1x { $(#[test] fn $name() { super::check(stringify!($name), 1); })* }
        mod gpu_at_2x { $(#[test] fn $name() { super::check(stringify!($name), 2); })* }
    };
}

paint_tests![
    clipping,
    icon_curves_paint,
    icons_paint,
    outline_paint,
    paint_boxes,
    image_paint,
    stacking,
];

#[test]
fn gpu_every_paint_fixture_has_a_test() {
    let missing: Vec<_> = paint_fixtures()
        .into_iter()
        .map(|f| f.name)
        .filter(|name| !TESTED.contains(name))
        .collect();
    assert!(
        missing.is_empty(),
        "paint fixtures with no test: {missing:?}; add them to `paint_tests!` in gpu_paint.rs"
    );
}
