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
#[path = "paint_support/placement.rs"]
mod placement;

use std::path::PathBuf;

use paint_support::{HEIGHT, WIDTH, browser, chromiums, image, laid_out, paint_fixtures, regions};
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
        multisampled: None,
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

/// Panics when a `lane_` fixture has no framed lane, so its comparison
/// would hold nothing of lane frames (`docs/plans/splice-layout.md`,
/// section 11).
fn assert_lanes_framed(name: &str, doc: &Document) {
    let framed = doc.update_counts().lanes_framed;
    assert!(
        !name.starts_with("lane_") || framed > 0,
        "{name} frames no lane: its rows are not whole device pixels, or too few; \
         fix the fixture (fixtures/{name}.wisp)"
    );
}

/// `name` at `scale` inks each element's region over the same rows and
/// columns as Chromium (`placement.rs`).
fn check_placement(name: &str, scale: u32) {
    let fixture = paint_fixtures()
        .into_iter()
        .find(|f| f.name == name)
        .expect("a paint fixture");
    let theirs = chromiums(&mut browser(), "placement", name, scale, "");
    let doc = laid_out(&fixture, scale);
    let ours = ours(&doc.borrow(), scale);
    let regions = regions(&doc.borrow(), scale);
    let (ours, theirs) = (
        placement::inked(&ours, &regions),
        placement::inked(&theirs, &regions),
    );
    let off: Vec<String> = ours
        .iter()
        .zip(&theirs)
        .filter(|(a, b)| a != b)
        .map(|((label, a), (_, b))| format!("{label}: ours {a:?}, Chromium's {b:?}"))
        .collect();
    assert!(
        off.is_empty(),
        "{name} at {scale}x places ink elsewhere than Chromium: {}",
        off.join("; ")
    );
}

fn check(name: &str, scale: u32) {
    let (found, out) = differences(name, scale);
    assert!(
        found.is_empty(),
        "{name} at {scale}x ({}) differs from Chromium's: {}",
        out.display(),
        found[..found.len().min(6)].join("; ")
    );
}

/// The tripwire for an `only_1x` fixture: panics once it matches
/// Chromium pixel for pixel at `scale`, so its relaxation comes off.
fn check_still_differs(name: &str, scale: u32) {
    let (found, out) = differences(name, scale);
    assert!(
        !found.is_empty(),
        "{name} now matches Chromium pixel for pixel at {scale}x ({}): move it out of `only_1x` in gpu_paint.rs and close the issue its reason names",
        out.display()
    );
}

/// Where `name` at `scale` differs from Chromium's, and the composite
/// written for a look.
fn differences(name: &str, scale: u32) -> (Vec<String>, PathBuf) {
    let fixture = paint_fixtures()
        .into_iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| {
            panic!("{name} is not a paint fixture: remove its test in gpu_paint.rs")
        });
    // Given
    let theirs = exact::with_exact_pixels(
        name,
        scale,
        chromiums(&mut browser(), "match", name, scale, ""),
    );
    let doc = laid_out(&fixture, scale);
    assert_lanes_framed(name, &doc.borrow());

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
    (found, out)
}

/// One test per fixture and scale, so nextest runs them in parallel and
/// each stays inside the 60 s deadline of `.config/nextest.toml`
/// (30 s period, terminated after 2). The `gpu_` prefix keeps them in the
/// gpu tier, which selects `test(/^gpu_/)`. `gpu_every_paint_fixture_has_a_test`
/// holds this list to the fixtures marked `/* paint`.
/// Fixtures after `only_1x:` are compared pixel by pixel at 1x alone, and
/// at 2x for placement only (the rows and columns that hold ink); each is
/// declared there with its reason and the issue that retires it, and
/// `gpu_at_2x_still_differs` fails once it matches at 2x.
macro_rules! paint_tests {
    ($($name:ident),* $(,)? ; only_1x: $($one:ident),* $(,)?) => {
        const TESTED: &[&str] = &[$(stringify!($name),)* $(stringify!($one)),*];
        mod gpu_at_1x {
            $(#[test] fn $name() { super::check(stringify!($name), 1); })*
            $(#[test] fn $one() { super::check(stringify!($one), 1); })*
        }
        mod gpu_at_2x { $(#[test] fn $name() { super::check(stringify!($name), 2); })* }
        mod gpu_at_2x_placement { $(#[test] fn $one() { super::check_placement(stringify!($one), 2); })* }
        mod gpu_at_2x_still_differs { $(#[test] fn $one() { super::check_still_differs(stringify!($one), 2); })* }
    };
}

paint_tests![
    clipping,
    icon_curves_paint,
    icon_paint_css,
    icons_paint,
    lane_list_paint,
    outline_paint,
    paint_boxes,
    image_paint,
    stacking,
    svg_shapes_paint;
    only_1x:
    // Relaxed at 2x to placement only (gpu_at_2x_placement holds every
    // glyph and box where Chromium puts them): the tops of the digits are
    // anti-aliased lighter than Chromium's. Retired by #388.
    text_line_box_paint,
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
