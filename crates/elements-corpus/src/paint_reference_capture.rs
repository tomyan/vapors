//! Capturing a saved reference again (saved references R1): the pinned
//! Chromium screenshots the fixture's page as the live paint tests do,
//! and its PNG is written under `fixtures/references/` byte for byte,
//! with a manifest of what it was captured from. `cargo xtask reference`
//! runs it, through the `paint_reference` example.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use vapors_text::comparison::Chromium;

use crate::oracle::{write_fonts, write_images};
use crate::paint_reference::{HEIGHT, WIDTH, references_dir, stem};
use crate::paint_reference_inputs::{expected, page, pinned, sha256, sha256_file};

/// Screenshots fixture `name` at `scale` with the pinned Chromium and
/// writes its PNG and manifest under `fixtures/references/`; the PNG's
/// path.
///
/// # Errors
/// When the Chromium found is not the pinned build (its sha256 differs
/// from `.config/tools.toml`'s), the screenshot fails, or a file cannot
/// be written.
pub fn capture(name: &str, scale: u32) -> Result<PathBuf, String> {
    let chromium = Chromium::find().map_err(|e| e.to_string())?;
    refuse_unpinned(chromium.binary())?;
    let work = tempfile::Builder::new()
        .prefix("vapors-paint-reference-")
        .tempdir()
        .map_err(|e| format!("a work directory: {e}"))?;
    let png = screenshot(&chromium, work.path(), name, scale)?;
    let bytes = std::fs::read(&png).map_err(|e| format!("{}: {e}", png.display()))?;
    let mut manifest = expected(name, scale)?;
    manifest.insert("png_sha256".into(), json!(sha256(&bytes)?));
    manifest.insert("host".into(), json!(host()));
    write(name, scale, &bytes, &Value::Object(manifest))
}

/// Refuses a browser other than the pinned build: a reference records
/// the pin, so one captured by another build would claim what it is not.
fn refuse_unpinned(binary: &Path) -> Result<(), String> {
    let found = sha256_file(binary)?;
    let pin = pinned("binary_sha256")?;
    if found == pin {
        return Ok(());
    }
    Err(format!(
        "{} has sha256 {found}, not the pinned {pin} (.config/tools.toml); \
         fix: unset VAPORS_CHROMIUM and run cargo xtask setup",
        binary.display()
    ))
}

/// Chromium's screenshot of the page in `work`, beside the fonts and
/// images it loads.
fn screenshot(chromium: &Chromium, work: &Path, name: &str, scale: u32) -> Result<PathBuf, String> {
    write_fonts(work)?;
    write_images(work)?;
    let html = work.join(format!("{name}.html"));
    std::fs::write(&html, page(name)?).map_err(|e| format!("{}: {e}", html.display()))?;
    let png = work.join(format!("{}.png", stem(name, scale)));
    chromium
        .screenshot(&html, &png, WIDTH, HEIGHT, scale)
        .map_err(|e| e.to_string())?;
    Ok(png)
}

fn write(name: &str, scale: u32, png: &[u8], manifest: &Value) -> Result<PathBuf, String> {
    let dir = references_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let png_path = dir.join(format!("{}.png", stem(name, scale)));
    std::fs::write(&png_path, png).map_err(|e| format!("{}: {e}", png_path.display()))?;
    let text = serde_json::to_string_pretty(manifest).map_err(|e| e.to_string())? + "\n";
    let manifest_path = dir.join(format!("{}.json", stem(name, scale)));
    std::fs::write(&manifest_path, text)
        .map_err(|e| format!("{}: {e}", manifest_path.display()))?;
    Ok(png_path)
}

/// The capture machine, for a reader: its OS and kernel, each saying so
/// when it cannot be read.
fn host() -> String {
    let os = std::fs::read_to_string("/etc/os-release").map_or_else(
        |e| format!("OS unreadable ({e})"),
        |text| {
            text.lines()
                .find_map(|line| line.strip_prefix("PRETTY_NAME="))
                .map_or("OS unnamed in /etc/os-release".to_owned(), |name| {
                    name.trim_matches('"').to_owned()
                })
        },
    );
    let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease").map_or_else(
        |e| format!("kernel unreadable ({e})"),
        |text| format!("Linux {}", text.trim()),
    );
    format!("{os}, {kernel}")
}
