//! Capturing saved references with Chromium (`paint_reference`): the
//! fixture's page, with its state's CSS appended, screenshotted at the
//! paint window and written under `fixtures/references/` byte for byte
//! as Chromium wrote it. Run on request only, by `cargo xtask reference`
//! through the `paint_reference` example; the result is looked at before
//! it is committed.

use std::path::{Path, PathBuf};

use vapors_text::comparison::Chromium;

use crate::oracle::{fixtures_dir, write_fonts, write_images};
use crate::page::{html, split};
use crate::paint_reference::{HEIGHT, Reference, WIDTH, references_dir};

/// Screenshots `reference` with Chromium into `fixtures/references/`;
/// the PNG's path.
///
/// # Errors
/// As [`capture_into`].
pub fn capture(reference: &Reference) -> Result<PathBuf, String> {
    capture_into(&references_dir(), reference)
}

/// As [`capture`], writing into `dir`.
///
/// # Errors
/// When no Chromium is found, the fixture cannot be read, the screenshot
/// fails, or a file cannot be written.
pub fn capture_into(dir: &Path, reference: &Reference) -> Result<PathBuf, String> {
    let chromium = Chromium::find().map_err(|e| e.to_string())?;
    let work = tempfile::Builder::new()
        .prefix("vapors-paint-reference-")
        .tempdir()
        .map_err(|e| format!("a work directory: {e}"))?;
    let png = screenshot(&chromium, work.path(), reference)?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let out = dir.join(reference.file_name());
    std::fs::copy(&png, &out)
        .map_err(|e| format!("{} to {}: {e}", png.display(), out.display()))?;
    Ok(out)
}

/// The page Chromium loads for `reference`: the fixture's, with its
/// state's CSS appended to the style.
///
/// # Errors
/// When the fixture cannot be read or split.
pub fn page(reference: &Reference) -> Result<String, String> {
    let path = fixtures_dir().join(format!("{}.wisp", reference.fixture));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut source = split(&reference.fixture, &text)?;
    if let Some(state) = reference.state {
        source.style.push_str(state.css);
    }
    Ok(html(&source))
}

/// Chromium's screenshot of `reference`'s page in `work`, beside the
/// fonts and images it loads.
fn screenshot(chromium: &Chromium, work: &Path, reference: &Reference) -> Result<PathBuf, String> {
    write_fonts(work)?;
    write_images(work)?;
    let html = work.join("page.html");
    std::fs::write(&html, page(reference)?).map_err(|e| format!("{}: {e}", html.display()))?;
    let png = work.join(reference.file_name());
    chromium
        .screenshot(&html, &png, WIDTH, HEIGHT, reference.scale)
        .map_err(|e| e.to_string())?;
    Ok(png)
}

#[cfg(test)]
#[path = "paint_reference_capture_tests.rs"]
mod tests;
