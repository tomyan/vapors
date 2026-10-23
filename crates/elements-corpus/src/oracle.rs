//! Chromium's boxes for each fixture: build its page, load it in
//! headless Chromium, read the boxes back, and write or compare
//! `fixtures/<name>.boxes.json`.

use std::path::{Path, PathBuf};

use serde_json::Value;
use vapors_text::comparison::{Chromium, FONT_FILES};

use crate::page::{FixtureViewport, Source, boxes_from_dom, framed, split};

/// The fixtures directory of this crate.
#[must_use]
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

/// The fixture names, sorted: the stems of `fixtures/*.wisp`.
///
/// # Errors
/// When the directory cannot be read.
pub fn fixture_names() -> Result<Vec<String>, String> {
    let dir = fixtures_dir();
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .file_name()
                .to_str()?
                .strip_suffix(".wisp")
                .map(str::to_owned)
        })
        .collect();
    names.sort();
    Ok(names)
}

/// Chromium's boxes for fixture `name` at each of its viewports, as the
/// JSON the corpus holds: `{"viewports": [{"viewport": [w, h], "scale": s,
/// "boxes": [..]}, ..]}`.
///
/// # Errors
/// A fixture that does not split, a page that cannot be written, or a
/// Chromium run that fails, writes no boxes, or lays the page out at
/// another size than asked.
pub fn boxes(chromium: &Chromium, work: &Path, name: &str) -> Result<Value, String> {
    let path = fixtures_dir().join(format!("{name}.wisp"));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let source = split(name, &text)?;
    let mut viewports = Vec::new();
    for viewport in &source.viewports {
        viewports.push(boxes_at(chromium, work, name, &source, *viewport)?);
    }
    Ok(serde_json::json!({ "viewports": viewports }))
}

fn boxes_at(
    chromium: &Chromium,
    work: &Path,
    name: &str,
    source: &Source,
    viewport: FixtureViewport,
) -> Result<Value, String> {
    let FixtureViewport {
        width: w,
        height: h,
        scale,
    } = viewport;
    let page = work.join(format!("{name}_{w}x{h}@{scale}.html"));
    std::fs::write(&page, framed(source, viewport))
        .map_err(|e| format!("{}: {e}", page.display()))?;
    // The window is the iframe's size, with room, and at least the
    // width headless Chromium will make a window.
    let dom = chromium
        .dump_dom(&page, w.max(500) + 40, h + 40, scale)
        .map_err(|e| format!("{name}: {e}"))?;
    let measured: Value =
        serde_json::from_str(&boxes_from_dom(&dom).map_err(|e| format!("{name}: {e}"))?)
            .map_err(|e| format!("{name}: the page's boxes are not JSON: {e}"))?;
    check_viewport(name, &measured, viewport)?;
    check_scale(name, &measured, scale)?;
    Ok(measured)
}

/// The fixture must have been laid out at exactly the size it asked for.
fn check_viewport(name: &str, page: &Value, wanted: FixtureViewport) -> Result<(), String> {
    let measured = [page["viewport"][0].as_f64(), page["viewport"][1].as_f64()];
    if measured
        == [
            Some(f64::from(wanted.width)),
            Some(f64::from(wanted.height)),
        ]
    {
        return Ok(());
    }
    Err(format!(
        "{name}: Chromium laid the fixture out at {measured:?}, not the {}x{} it asks for",
        wanted.width, wanted.height
    ))
}

/// The page's `devicePixelRatio` must be the scale the fixture asks for:
/// boxes laid out at another would be compared at the wrong one.
fn check_scale(name: &str, page: &Value, scale: f64) -> Result<(), String> {
    let measured = page["scale"]
        .as_f64()
        .ok_or(format!("{name}: the page wrote no scale"))?;
    if (measured - scale).abs() < 1e-6 {
        return Ok(());
    }
    Err(format!(
        "{name}: Chromium laid the page out at device scale {measured}, not the {scale} the fixture asks for; check that --force-device-scale-factor reaches it"
    ))
}

/// Writes the bundled fonts into `work`, beside the pages.
///
/// # Errors
/// When a file cannot be written.
pub fn write_fonts(work: &Path) -> Result<(), String> {
    for (file, bytes) in FONT_FILES {
        let path = work.join(file);
        std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

/// The corpus file text for `boxes`: one box per line, so a change
/// reads as a one-line diff.
#[must_use]
pub fn to_text(fixture: &Value) -> String {
    let sections: Vec<String> = fixture["viewports"]
        .as_array()
        .map(|sections| sections.iter().map(section_text).collect())
        .unwrap_or_default();
    format!("{{\"viewports\": [\n{}\n]}}\n", sections.join(",\n"))
}

fn section_text(boxes: &Value) -> String {
    let rows: Vec<String> = boxes["boxes"]
        .as_array()
        .map(|rows| rows.iter().map(Value::to_string).collect())
        .unwrap_or_default();
    format!(
        "{{\"viewport\": {}, \"scale\": {},\n\"boxes\": [\n{}\n]}}",
        boxes["viewport"],
        boxes["scale"],
        rows.join(",\n")
    )
}
