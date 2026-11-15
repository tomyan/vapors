//! The report's Markdown: per placement and method, how many of the
//! icons pass the `paint_check` rules against Chromium, and for each that
//! fails its pixels over the rules, its strongly-off count and its
//! largest difference.

use std::path::Path;

use vapors_text::comparison::{Rgba, write_png};

use crate::cases::{CELL, Icon, Placement};
use crate::compare::{Found, cell};

/// The browser and adapter the run used.
pub fn identity(gpu: &vapors_gpu_harness::Gpu) -> String {
    let chrome = vapors_text::comparison::Chromium::find().unwrap_or_else(|e| panic!("{e}"));
    let info = gpu.adapter.get_info();
    format!(
        "Chromium: {} ({})\n\nAdapter: {} ({:?}; {} {})\n",
        chrome.version(),
        chrome.binary().display(),
        info.name,
        info.backend,
        info.driver,
        info.driver_info
    )
}

pub const HEADER: &str = "| placement | method | pass | failing icons (pixels over rules 1-2 / strongly off / max difference) |\n|---|---|---|---|";
pub const CLOSED_HEADER: &str = "\n| placement | method (closed forms) | pass | each shape (pixels over rules 1-2 / strongly off / max difference) |\n|---|---|---|---|";
pub const LIMITS_HEADER: &str = "\n| case | method | pass | detail |\n|---|---|---|---|";
pub const TOLERANCE_HEADER: &str =
    "\n| placement | tolerance (px) | undo | redo |\n|---|---|---|---|";

pub fn row(
    placement: &Placement,
    method: &str,
    icons: &[&Icon],
    ours: Option<&[u8]>,
    theirs: &Rgba,
    out: Option<&Path>,
) {
    described_row(placement, method, icons, ours, theirs, out, failing);
}

/// As [`row`], with every shape's numbers rather than the failing ones'.
pub fn closed_row(
    placement: &Placement,
    method: &str,
    icons: &[&Icon],
    ours: &[u8],
    theirs: &Rgba,
    out: Option<&Path>,
) {
    described_row(placement, method, icons, Some(ours), theirs, out, every);
}

fn described_row(
    placement: &Placement,
    method: &str,
    icons: &[&Icon],
    ours: Option<&[u8]>,
    theirs: &Rgba,
    out: Option<&Path>,
    describe: fn(&[&Icon], &[Found]) -> String,
) {
    let Some(ours) = ours else {
        println!(
            "| {} | {method} | n/a | draws axis-aligned at whole pixels only |",
            placement.name
        );
        return;
    };
    save(
        out,
        &format!("{}-{method}", placement.name),
        ours,
        theirs.width,
    );
    let found = judged(ours, theirs, icons.len(), placement.scale, placement.name);
    let passed = found.iter().filter(|f| f.pass).count();
    println!(
        "| {} | {method} | {passed}/{} | {} |",
        placement.name,
        icons.len(),
        describe(icons, &found)
    );
}

pub fn limit_row(
    case: &str,
    method: &str,
    icons: &[&Icon],
    ours: Option<&[u8]>,
    theirs: &Rgba,
    out: Option<&Path>,
) {
    let Some(ours) = ours else {
        println!("| {case} | {method} | n/a | |");
        return;
    };
    save(out, &format!("{case}-{method}"), ours, theirs.width);
    let found = judged(ours, theirs, icons.len(), 1, case);
    let passed = found.iter().filter(|f| f.pass).count();
    println!(
        "| {case} | {method} | {passed}/{} | {} |",
        icons.len(),
        failing(icons, &found)
    );
}

pub fn tolerance_row(
    placement: &Placement,
    tolerance: f32,
    icons: &[&Icon],
    ours: &[u8],
    theirs: &Rgba,
) {
    let found = judged(ours, theirs, icons.len(), placement.scale, placement.name);
    let text: Vec<String> = found.iter().map(summary).collect();
    println!(
        "| {} | {tolerance} | {} | {} |",
        placement.name, text[0], text[1]
    );
}

/// Each cell of `ours` against `theirs`, refusing images of other sizes
/// than `count` cells at `scale`, which would compare the wrong pixels.
fn judged(ours: &[u8], theirs: &Rgba, count: usize, scale: u32, what: &str) -> Vec<Found> {
    let side = CELL * scale;
    let cells = u32::try_from(count).expect("a few");
    assert!(
        theirs.width == side * cells && theirs.height == side,
        "{what}: Chromium's image is {} x {}, not {} x {side}; rerun the example",
        theirs.width,
        theirs.height,
        side * cells
    );
    assert_eq!(
        ours.len(),
        theirs.data.len(),
        "{what}: our image has {} bytes and Chromium's {}; the method drew another size",
        ours.len(),
        theirs.data.len()
    );
    (0..u32::try_from(count).expect("a few"))
        .map(|at| cell(ours, &theirs.data, theirs.width, side, at))
        .collect()
}

fn failing(icons: &[&Icon], found: &[Found]) -> String {
    let failed: Vec<String> = icons
        .iter()
        .zip(found)
        .filter(|(_, f)| !f.pass)
        .map(|(icon, found)| format!("{} ({})", icon.name, summary(found)))
        .collect();
    failed.join("; ")
}

fn every(icons: &[&Icon], found: &[Found]) -> String {
    let each: Vec<String> = icons
        .iter()
        .zip(found)
        .map(|(icon, found)| format!("{} ({})", icon.name, summary(found)))
        .collect();
    each.join("; ")
}

fn summary(found: &Found) -> String {
    let strong = found.strong.map_or("≤2".to_owned(), |n| n.to_string());
    let verdict = if found.pass { "pass" } else { "fail" };
    format!("{verdict} {} / {strong} / {}", found.pixels, found.max)
}

pub fn save(out: Option<&Path>, name: &str, rgba: &[u8], width: u32) {
    let Some(out) = out else { return };
    let height = u32::try_from(rgba.len() / 4).expect("pixels") / width;
    let image = Rgba {
        width,
        height,
        data: rgba.to_vec(),
    };
    write_png(&out.join(format!("{}.png", name.replace(' ', "_"))), &image)
        .expect("the png is written");
}
