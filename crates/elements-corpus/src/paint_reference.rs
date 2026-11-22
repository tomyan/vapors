//! A saved Chromium reference for a paint fixture (saved references R1,
//! `docs/plans/saved-chromium-references.md`): Chromium's screenshot of
//! the fixture, committed under `fixtures/references/` with a manifest of
//! what it was captured from. The paint tests read it in place of a live
//! screenshot, after refusing it when it is missing or when anything it
//! was captured from has changed since (`paint_reference_inputs`).
//! `cargo xtask reference <fixture> <scale>` captures it again
//! (`paint_reference_capture`).

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use vapors_text::comparison::{Rgba, read_png};

use crate::oracle::fixtures_dir;
pub use crate::paint_reference_capture::capture;
pub use crate::paint_reference_inputs::{expected, pinned, sha256};

/// The paint fixtures' window, in CSS pixels: the live screenshots' and
/// the saved references'.
pub const WIDTH: u32 = 640;
/// See [`WIDTH`].
pub const HEIGHT: u32 = 400;

/// The manifest fields that are not checked: the capture machine, which
/// is expected to differ from the one checking.
const UNCHECKED: &[&str] = &["host"];

/// Where the saved references are committed.
#[must_use]
pub fn references_dir() -> PathBuf {
    fixtures_dir().join("references")
}

/// The saved reference of fixture `name` at `scale`, checked against the
/// tree as it is now.
///
/// # Errors
/// As [`check`], or when the tree's inputs cannot be read.
pub fn saved(name: &str, scale: u32) -> Result<Rgba, String> {
    check(&references_dir(), name, scale, &expected(name, scale)?)
}

/// The reference of `name` at `scale` in `dir`, when its manifest holds
/// `expected`'s fields and its PNG has the manifest's hash and size.
///
/// # Errors
/// A missing or malformed manifest, a field it lacks, differs in or does
/// not recognise, a missing PNG or one of another hash or size; each
/// names the file and the command that captures it again.
pub fn check(
    dir: &Path,
    name: &str,
    scale: u32,
    expected: &Map<String, Value>,
) -> Result<Rgba, String> {
    let fix = |path: &Path, why: String| refusal(path, &why, name, scale);
    let manifest_path = dir.join(format!("{}.json", stem(name, scale)));
    let manifest = read_manifest(&manifest_path).map_err(|why| fix(&manifest_path, why))?;
    stale(&manifest, expected).map_err(|why| fix(&manifest_path, why))?;
    let png_path = dir.join(format!("{}.png", stem(name, scale)));
    let image = read_checked_png(&png_path, &manifest).map_err(|why| fix(&png_path, why))?;
    Ok(image)
}

/// The command that captures `name` at `scale` again.
#[must_use]
pub fn regenerate_command(name: &str, scale: u32) -> String {
    format!("cargo xtask reference {name} {scale}")
}

/// The file stem of `name` at `scale`, as `text_line_box_paint_2x`.
#[must_use]
pub fn stem(name: &str, scale: u32) -> String {
    format!("{name}_{scale}x")
}

fn refusal(path: &Path, why: &str, name: &str, scale: u32) -> String {
    format!(
        "{}: {why}; fix: run `{}` with the pinned Chromium installed (cargo xtask setup), \
         look at the PNG, and commit it with its manifest",
        path.display(),
        regenerate_command(name, scale)
    )
}

fn read_manifest(path: &Path) -> Result<Map<String, Value>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("no saved reference ({e})"))?;
    match serde_json::from_str(&text) {
        Ok(Value::Object(manifest)) => Ok(manifest),
        Ok(_) => Err("the manifest is not a JSON object".to_owned()),
        Err(e) => Err(format!("the manifest is not JSON ({e})")),
    }
}

/// Every field the manifest lacks, differs in, or does not recognise.
fn stale(manifest: &Map<String, Value>, expected: &Map<String, Value>) -> Result<(), String> {
    let mut found: Vec<String> = expected
        .iter()
        .filter(|(field, now)| manifest.get(*field) != Some(now))
        .map(|(field, now)| match manifest.get(field) {
            None => format!("no `{field}` (now {now})"),
            Some(saved) => format!("`{field}` is {saved}, now {now}"),
        })
        .collect();
    found.extend(
        manifest
            .keys()
            .filter(|field| !expected.contains_key(*field) && !is_known(field))
            .map(|field| format!("`{field}` is not a manifest field")),
    );
    if found.is_empty() {
        return Ok(());
    }
    Err(format!("stale: {}", found.join("; ")))
}

fn is_known(field: &str) -> bool {
    UNCHECKED.contains(&field) || field == "png_sha256"
}

/// The PNG at `path`, when its hash is the manifest's `png_sha256` and
/// its size the manifest's window at its scale.
fn read_checked_png(path: &Path, manifest: &Map<String, Value>) -> Result<Rgba, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("no saved PNG ({e})"))?;
    let hash = sha256(&bytes)?;
    let saved = manifest.get("png_sha256").and_then(Value::as_str);
    if saved != Some(hash.as_str()) {
        return Err(format!("its sha256 is {hash}, the manifest's {saved:?}"));
    }
    let image = read_png(path)?;
    let size =
        expected_size(manifest).ok_or("the manifest gives no whole width, height and scale")?;
    if [image.width, image.height] != size {
        return Err(format!(
            "it is {}x{}, the manifest's {}x{}",
            image.width, image.height, size[0], size[1]
        ));
    }
    Ok(image)
}

fn expected_size(manifest: &Map<String, Value>) -> Option<[u32; 2]> {
    let field = |name: &str| {
        manifest
            .get(name)?
            .as_u64()
            .and_then(|v| u32::try_from(v).ok())
    };
    let scale = field("scale")?;
    Some([field("width")? * scale, field("height")? * scale])
}

#[cfg(test)]
#[path = "paint_reference_tests.rs"]
mod tests;
