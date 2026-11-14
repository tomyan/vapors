//! S3b's antialiasing measurement (docs/plans/one-scene.md, the S3b
//! row): touche's ten icons drawn as shapes by each method beside
//! Chromium, at 1x, 2x, rotated 30 degrees and scaled 1.37, each cell
//! judged by the `paint_check` rules; the known limits' cases; and the
//! flattening tolerance on the icons with arcs. Prints a Markdown
//! report; `--out <dir>` also writes every image as PNG.
//!
//! `cargo run -p vapors-elements-corpus --example aa_methods [-- --out <dir>]`

// A measurement program, not shipped code: its pixel coordinates and
// counts are far below f32's 2^24, colour channels are rounded into
// 0..=255 on purpose, and matrices are unpacked by CSS's single-letter
// names. Retired with the program once S3b's choice is built and tested.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

mod analytic;
mod cases;
mod chromium_page;
mod closed;
mod compare;
mod corners;
mod icon_route;
mod msaa;
mod offscreen;
mod report;

use std::path::PathBuf;

use cases::{
    FAINT, ICONS, INK, Icon, Placement, SUB_PIXEL, closed_cases, placements, sub_pixel_closed_cases,
};

/// The flattening tolerance the methods draw at, device pixels.
const TOLERANCE_PX: f32 = 0.1;

fn main() {
    let out = out_dir();
    let gpu = vapors_gpu_harness::gpu();
    let work = tempfile::tempdir().expect("a work directory");
    println!("{}", report::identity(&gpu));
    println!("{}", report::HEADER);
    let icons: Vec<&Icon> = ICONS.iter().collect();
    for placement in placements() {
        let theirs = chromium_page::drawn(&icons, &placement, INK, work.path());
        report::save(
            out.as_deref(),
            &format!("{}-chromium", placement.name),
            &theirs.data,
            theirs.width,
        );
        for (method, ours) in methods(&gpu, &icons, &placement, INK) {
            report::row(
                &placement,
                method,
                &icons,
                ours.as_deref(),
                &theirs,
                out.as_deref(),
            );
        }
    }
    closed_forms(&gpu, work.path(), out.as_deref());
    limits(&gpu, work.path(), out.as_deref());
    tolerances(&gpu, work.path());
}

/// Each method's drawing of `icons`, `None` where it cannot draw them.
fn methods(
    gpu: &vapors_gpu_harness::Gpu,
    icons: &[&Icon],
    placement: &Placement,
    ink: [u8; 4],
) -> Vec<(&'static str, Option<Vec<u8>>)> {
    vec![
        (
            "analytic",
            Some(analytic::drawn(gpu, icons, placement, ink, TOLERANCE_PX)),
        ),
        (
            "MSAA 4x",
            Some(msaa::drawn(gpu, icons, placement, ink, TOLERANCE_PX)),
        ),
        (
            "exact corners",
            Some(corners::drawn(gpu, icons, placement, ink, TOLERANCE_PX)),
        ),
        ("icon route", icon_route::drawn(icons, placement, ink)),
    ]
}

/// The closed-form kind: two circles and two rects, by the closed-form
/// pipeline and, from their paths, by the mesh methods.
fn closed_forms(
    gpu: &vapors_gpu_harness::Gpu,
    work: &std::path::Path,
    out: Option<&std::path::Path>,
) {
    println!("{}", report::CLOSED_HEADER);
    let cases = closed_cases();
    let shapes: Vec<&Icon> = cases.iter().map(|case| &case.shape).collect();
    for placement in placements() {
        let theirs = chromium_page::drawn(&shapes, &placement, INK, work);
        let ours = closed::drawn(gpu, &cases, &placement, INK);
        report::row(
            &placement,
            "closed form",
            &shapes,
            Some(&ours),
            &theirs,
            out,
        );
        let msaa = msaa::drawn(gpu, &shapes, &placement, INK, TOLERANCE_PX);
        report::row(&placement, "MSAA 4x", &shapes, Some(&msaa), &theirs, out);
        let corners = corners::drawn(gpu, &shapes, &placement, INK, TOLERANCE_PX);
        report::row(
            &placement,
            "exact corners",
            &shapes,
            Some(&corners),
            &theirs,
            out,
        );
    }
}

/// The known limits' cases at 1x: a quarter-pixel stroke, as a path and
/// as closed forms, and Tools and Paste in translucent paint where their
/// strokes overlap.
fn limits(gpu: &vapors_gpu_harness::Gpu, work: &std::path::Path, out: Option<&std::path::Path>) {
    let one = placements()[0];
    println!("{}", report::LIMITS_HEADER);
    for (name, icons, ink) in [
        ("sub-pixel stroke", vec![&SUB_PIXEL], INK),
        ("translucent overlap", vec![&ICONS[5], &ICONS[1]], FAINT),
    ] {
        let theirs = chromium_page::drawn(&icons, &one, ink, work);
        for (method, ours) in methods(gpu, &icons, &one, ink) {
            report::limit_row(name, method, &icons, ours.as_deref(), &theirs, out);
        }
    }
    sub_pixel_closed(gpu, work, out);
}

/// A circle and a rect stroked 0.25 px at 1x, by the closed-form
/// pipeline and the mesh methods.
fn sub_pixel_closed(
    gpu: &vapors_gpu_harness::Gpu,
    work: &std::path::Path,
    out: Option<&std::path::Path>,
) {
    let (one, name) = (placements()[0], "sub-pixel closed forms");
    let cases = sub_pixel_closed_cases();
    let shapes: Vec<&Icon> = cases.iter().map(|case| &case.shape).collect();
    let theirs = chromium_page::drawn(&shapes, &one, INK, work);
    let closed = closed::drawn(gpu, &cases, &one, INK);
    report::limit_row(name, "closed form", &shapes, Some(&closed), &theirs, out);
    for (method, ours) in methods(gpu, &shapes, &one, INK) {
        report::limit_row(name, method, &shapes, ours.as_deref(), &theirs, out);
    }
}

/// The analytic method on Undo and Redo (the icons with arcs) at three
/// flattening tolerances, under each placement.
fn tolerances(gpu: &vapors_gpu_harness::Gpu, work: &std::path::Path) {
    println!("{}", report::TOLERANCE_HEADER);
    let icons = vec![&ICONS[3], &ICONS[4]];
    for placement in placements() {
        let theirs = chromium_page::drawn(&icons, &placement, INK, work);
        for tolerance in [0.25, 0.1, 0.05] {
            let ours = analytic::drawn(gpu, &icons, &placement, INK, tolerance);
            report::tolerance_row(&placement, tolerance, &icons, &ours, &theirs);
        }
    }
}

fn out_dir() -> Option<PathBuf> {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next()) {
        (Some("--out"), Some(dir)) => {
            std::fs::create_dir_all(&dir).expect("the out directory");
            Some(PathBuf::from(dir))
        }
        (None, _) => None,
        _ => panic!("usage: aa_methods [--out <dir>]"),
    }
}
