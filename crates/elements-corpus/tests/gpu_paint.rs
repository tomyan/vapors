//! GPU tier: each paint fixture (a `/* paint */` comment in its style),
//! drawn by vapors offscreen and by headless Chromium at 1x and 2x, is
//! held to the rules in `paint_check`: flat regions within 2, every
//! pixel within 96, and in each element's region at most 2 pixels off
//! by more than half the local contrast. The composite (vapors | Chromium | difference
//! x4) is written to `target/tmp/paint_<name>_<scale>x.png`.
//! The rules are checked in `gpu_paint_rules`.

mod paint_support;

use std::path::PathBuf;

use paint_support::{HEIGHT, WIDTH, chromiums, image, laid_out, paint_fixtures, regions};
use vapors_app::ScenePainter;
use vapors_elements::Document;
use vapors_elements_corpus::paint_check::compare;
use vapors_gpu_harness::{gpu, read_rgba8};
use vapors_renderer::{Target, clear};
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
    let target = Target {
        device: &gpu.device,
        queue: &gpu.queue,
        view: &view,
        viewport: size,
    };
    let painted = doc.painted().expect("painted");
    ScenePainter::new(&gpu.device, texture.format())
        .draw(&target, painted)
        .expect("paints");
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

fn check_at(scale: u32) {
    let mut failures = Vec::new();
    for fixture in paint_fixtures() {
        // Given
        let theirs = chromiums("match", fixture.name, scale, "");
        let doc = laid_out(&fixture, scale);

        // When
        let ours = ours(&doc.borrow(), scale);

        // Then
        let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("paint_{}_{scale}x.png", fixture.name));
        write_png(&out, &composite(&ours, &theirs)).expect("composite");
        let found = compare(
            &image(&ours),
            &image(&theirs),
            &regions(&doc.borrow(), scale),
        );
        if !found.is_empty() {
            failures.push(format!(
                "{} at {scale}x ({}): {}",
                fixture.name,
                out.display(),
                found[..found.len().min(6)].join("; ")
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
