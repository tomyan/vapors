//! Chromium's boxes for each fixture: build its page, load it in
//! headless Chromium, read the boxes back, and write or compare
//! `fixtures/<name>.boxes.json`.

use std::path::{Path, PathBuf};

use serde_json::Value;
use vapors_text::comparison::{Chromium, FONT_FILES};

use crate::page::{boxes_from_dom, html, split};

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

/// Chromium's boxes for fixture `name`, as the JSON the corpus holds.
///
/// # Errors
/// A fixture that does not split, a page that cannot be written, or a
/// Chromium run that fails or writes no boxes.
pub fn boxes(chromium: &Chromium, work: &Path, name: &str) -> Result<Value, String> {
    let path = fixtures_dir().join(format!("{name}.wisp"));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let source = split(name, &text)?;
    let page = work.join(format!("{name}.html"));
    std::fs::write(&page, html(&source)).map_err(|e| format!("{}: {e}", page.display()))?;
    let dom = chromium
        .dump_dom(&page, source.viewport[0], source.viewport[1], source.scale)
        .map_err(|e| format!("{name}: {e}"))?;
    let page: Value =
        serde_json::from_str(&boxes_from_dom(&dom).map_err(|e| format!("{name}: {e}"))?)
            .map_err(|e| format!("{name}: the page's boxes are not JSON: {e}"))?;
    check_viewport(name, &page, source.viewport[0])?;
    check_scale(name, &page, source.scale)?;
    Ok(page)
}

/// Chromium's window has a minimum width (500 px headless, 155 here):
/// a narrower fixture gets a wider page than it asked for. The height
/// is whatever the page measured: headless `--dump-dom` gives a 640x400
/// window a 313 px tall viewport, and the corpus lays out at that.
fn check_viewport(name: &str, page: &Value, width: u32) -> Result<(), String> {
    let html = page["viewport"][0]
        .as_f64()
        .ok_or(format!("{name}: the page wrote no viewport width"))?;
    if (html - f64::from(width)).abs() < 0.001 {
        return Ok(());
    }
    Err(format!(
        "{name}: Chromium laid the page out {html} px wide, not the {width} px the fixture asks for; Chromium's minimum window width is about 500 px, so give the fixture a viewport at least that wide"
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
pub fn to_text(boxes: &Value) -> String {
    let rows: Vec<String> = boxes["boxes"]
        .as_array()
        .map(|rows| rows.iter().map(Value::to_string).collect())
        .unwrap_or_default();
    format!(
        "{{\"viewport\": {}, \"scale\": {},\n\"boxes\": [\n{}\n]}}\n",
        boxes["viewport"],
        boxes["scale"],
        rows.join(",\n")
    )
}
