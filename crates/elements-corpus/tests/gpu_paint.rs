//! GPU tier: each paint fixture (a `/* paint */` comment in its style),
//! drawn by vapors offscreen and by headless Chromium at 1x and 2x, is
//! compared pixel by pixel.
//!
//! - Where Chromium's pixel and its eight neighbours are one colour
//!   (within 1), ours must be within 2: a misplaced edge, a wrong
//!   colour or a missing box fails here.
//! - Elsewhere (anti-aliased edges, border corner joins, shadow
//!   falloff) ours must be within `EDGE_BOUND`, and at most
//!   `EDGE_SHARE` of all pixels may differ by more than 2.
//!
//! Measured on `paint_boxes` with Chromium 155, 2026-10-23, on this
//! machine's Intel adapter (the bounds also hold on lavapipe): no
//! flat pixel off; the worst edge pixel off by 86 (a border corner
//! join, which Skia blends and vapors splits on the diagonal); 0.275%
//! of pixels off by more than 2 at 1x, 0.151% at 2x. The composite
//! (vapors | Chromium | difference x4) is written to
//! `target/tmp/paint_<name>_<scale>x.png`.

use std::path::PathBuf;

use vapors_app::ScenePainter;
use vapors_elements::{Document, Viewport};
use vapors_elements_corpus::oracle::{fixtures_dir, write_fonts};
use vapors_elements_corpus::page::{html, split};
use vapors_elements_corpus::{Fixture, all};
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{Target, clear};
use vapors_scene::{Color, Size};
use vapors_text::FontSystem;
use vapors_text::comparison::{Chromium, Rgba, write_png};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 400;
/// The largest difference an edge pixel may have: 86 measured, plus room.
const EDGE_BOUND: u8 = 96;
/// The share of pixels that may differ by more than 2: 0.275% measured.
const EDGE_SHARE: f64 = 0.005;

fn ours(fixture: &Fixture, scale: u32) -> Rgba {
    let doc = std::rc::Rc::new(std::cell::RefCell::new(Document::new(
        FontSystem::bundle_only(),
    )));
    doc.borrow_mut()
        .add_stylesheet(fixture.name, fixture.style)
        .expect("parses");
    let body = doc.borrow().body();
    (fixture.mount)(&doc, body);
    #[allow(clippy::cast_precision_loss)]
    let viewport = Viewport {
        width: WIDTH as f32,
        height: HEIGHT as f32,
        scale: scale as f32,
    };
    let mut doc = doc.borrow_mut();
    let painted = doc
        .update(viewport)
        .unwrap_or_else(|e| panic!("{}: refused: {e:#?}", fixture.name));
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
    let target = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        view: &view,
        viewport: size,
    };
    ScenePainter::new(&gpu.device, texture.format())
        .draw(&target, painted)
        .expect("paints");
    Rgba {
        width: size.width,
        height: size.height,
        data: read_rgba8(&gpu.device, &gpu.queue, &texture),
    }
}

fn chromiums(name: &str, scale: u32) -> Rgba {
    let work = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("paint-{name}"));
    std::fs::create_dir_all(&work).expect("work dir");
    write_fonts(&work).expect("fonts");
    let path = fixtures_dir().join(format!("{name}.wisp"));
    let source = split(name, &std::fs::read_to_string(&path).expect("fixture")).expect("splits");
    let page = work.join("page.html");
    std::fs::write(&page, html(&source)).expect("page");
    let chrome = Chromium::find().unwrap_or_else(|e| panic!("{e}"));
    let png = work.join(format!("chromium_{scale}x.png"));
    chrome
        .screenshot(&page, &png, WIDTH, HEIGHT, scale)
        .unwrap_or_else(|e| panic!("{e}"))
}

fn pixel(image: &Rgba, x: u32, y: u32) -> [u8; 3] {
    let at = ((y * image.width + x) * 4) as usize;
    [image.data[at], image.data[at + 1], image.data[at + 2]]
}

fn distance(a: [u8; 3], b: [u8; 3]) -> u8 {
    (0..3).map(|i| a[i].abs_diff(b[i])).max().unwrap_or(0)
}

/// Whether Chromium's pixel and its eight neighbours are one colour.
fn flat(theirs: &Rgba, x: u32, y: u32) -> bool {
    let centre = pixel(theirs, x, y);
    (x - 1..=x + 1).all(|i| (y - 1..=y + 1).all(|j| distance(pixel(theirs, i, j), centre) <= 1))
}

/// The failures of `ours` against `theirs` under the rules above.
fn compare(ours: &Rgba, theirs: &Rgba) -> Vec<String> {
    let (mut failures, mut off) = (Vec::new(), 0usize);
    for y in 1..theirs.height - 1 {
        for x in 1..theirs.width - 1 {
            let d = distance(pixel(ours, x, y), pixel(theirs, x, y));
            off += usize::from(d > 2);
            if (d > 2 && flat(theirs, x, y)) || d > EDGE_BOUND {
                failures.push(format!(
                    "({x}, {y}) ours {:?}, Chromium's {:?}",
                    pixel(ours, x, y),
                    pixel(theirs, x, y)
                ));
            }
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let share = off as f64 / f64::from(theirs.width * theirs.height);
    if share > EDGE_SHARE {
        failures.push(format!(
            "{:.3}% of pixels differ by more than 2; the bound is {:.3}%",
            share * 100.0,
            EDGE_SHARE * 100.0
        ));
    }
    failures
}

/// vapors | Chromium | difference x4, for eyes.
fn composite(ours: &Rgba, theirs: &Rgba) -> Rgba {
    let mut data = Vec::with_capacity(ours.data.len() * 3);
    for y in 0..ours.height {
        let row = |image: &Rgba| {
            image.data[(y * image.width * 4) as usize..((y + 1) * image.width * 4) as usize]
                .to_vec()
        };
        data.extend(row(ours));
        data.extend(row(theirs));
        for x in 0..ours.width {
            let d = distance(pixel(ours, x, y), pixel(theirs, x, y)).saturating_mul(4);
            data.extend([d, d, d, 255]);
        }
    }
    Rgba {
        width: ours.width * 3,
        height: ours.height,
        data,
    }
}

/// Each paint fixture at `scale`; one screenshot per fixture, so a test
/// stays well inside nextest's slow-timeout (a screenshot is about 1 s
/// here, under 10 s on a cold runner).
fn check_at(scale: u32) {
    let fixtures: Vec<Fixture> = all()
        .into_iter()
        .filter(|f| f.style.contains("/* paint"))
        .collect();
    assert!(
        !fixtures.is_empty(),
        "no paint fixtures: nothing is compared"
    );
    let mut failures = Vec::new();
    for fixture in &fixtures {
        // Given
        let theirs = chromiums(fixture.name, scale);

        // When
        let ours = ours(fixture, scale);

        // Then
        let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("paint_{}_{scale}x.png", fixture.name));
        write_png(&out, &composite(&ours, &theirs)).expect("composite");
        let found = compare(&ours, &theirs);
        if !found.is_empty() {
            failures.push(format!(
                "{} at {scale}x ({}): {} failures, first: {}",
                fixture.name,
                out.display(),
                found.len(),
                found[..found.len().min(5)].join("; ")
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "paint differs from Chromium's:\n{}",
        failures.join("\n")
    );
}

#[test]
fn gpu_paint_fixtures_match_chromiums_pixels_at_1x() {
    check_at(1);
}

#[test]
fn gpu_paint_fixtures_match_chromiums_pixels_at_2x() {
    check_at(2);
}
