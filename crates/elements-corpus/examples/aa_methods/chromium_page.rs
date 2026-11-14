//! Chromium's drawing of a row of icons: one page, one cell each, the
//! svg placed and transformed as `cases.rs` says, screenshotted.

use std::fmt::Write as _;
use std::path::Path;

use vapors_text::comparison::{Chromium, Rgba};

use crate::cases::{AT, CELL, Icon, PAGE, Placement};

/// `icons` drawn by Chromium in `ink` under `placement`, one cell each.
pub fn drawn(icons: &[&Icon], placement: &Placement, ink: [u8; 4], work: &Path) -> Rgba {
    let html = work.join(format!("{}.html", file_name(placement)));
    std::fs::write(&html, page(icons, placement, ink)).expect("the page is written");
    let png = work.join(format!("{}.png", file_name(placement)));
    let width = CELL * u32::try_from(icons.len()).expect("a few icons");
    let chrome = Chromium::find().unwrap_or_else(|e| panic!("{e}"));
    chrome
        .screenshot(&html, &png, width, CELL, placement.scale)
        .unwrap_or_else(|e| panic!("{e}"))
}

fn file_name(placement: &Placement) -> String {
    placement.name.replace(' ', "_")
}

fn page(icons: &[&Icon], placement: &Placement, ink: [u8; 4]) -> String {
    let [r, g, b, _] = PAGE;
    let colour = format!(
        "rgba({}, {}, {}, {})",
        ink[0],
        ink[1],
        ink[2],
        f32::from(ink[3]) / 255.0
    );
    let mut html = format!(
        "<!doctype html><style>html, body {{ margin: 0; background: rgb({r}, {g}, {b}); }} \
         svg {{ position: absolute; top: {AT}px; transform: {}; }}</style>",
        placement.css
    );
    for (at, icon) in icons.iter().enumerate() {
        let left = at as f32 * CELL as f32 + AT;
        let paint = if icon.stroke > 0.0 {
            format!(
                "fill=\"none\" stroke=\"{colour}\" stroke-width=\"{}\"",
                icon.stroke
            )
        } else {
            format!("fill=\"{colour}\"")
        };
        let _ = write!(
            html,
            "<svg style=\"left: {left}px\" viewBox=\"0 0 16 16\" width=\"16\" height=\"16\"><path d=\"{}\" {paint}/></svg>",
            icon.path
        );
    }
    html
}
