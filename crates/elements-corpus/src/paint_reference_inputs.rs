//! What a saved reference is captured from, as the tree holds it now
//! (saved references R1): the page, the fonts and images beside it, the
//! pinned Chromium, its render flags, and the window and scale. A saved
//! reference whose manifest records anything else is stale
//! (`paint_reference::check`).

use serde_json::{Map, Value, json};
use vapors_text::comparison::{FONT_FILES, render_flags};

use crate::oracle::{fixtures_dir, image_files};
use crate::page::{html, split};
use crate::paint_reference::{HEIGHT, WIDTH};
use crate::sha256::sha256;

/// The tools file, as built: the pin's one source, read as
/// `vapors_text`'s `chromium_pin` reads it.
const TOOLS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../.config/tools.toml"
));

/// The manifest fields of fixture `name` at `scale` as the tree holds
/// them now, all of them checked.
///
/// # Errors
/// When the fixture, an image or the pin cannot be read, or a hash
/// cannot be taken.
pub fn expected(name: &str, scale: u32) -> Result<Map<String, Value>, String> {
    let mut fields = Map::new();
    fields.insert("fixture".into(), json!(name));
    fields.insert("scale".into(), json!(scale));
    fields.insert("width".into(), json!(WIDTH));
    fields.insert("height".into(), json!(HEIGHT));
    fields.insert("page_sha256".into(), json!(sha256(page(name)?.as_bytes())?));
    fields.insert(
        "fonts".into(),
        hashes(FONT_FILES.iter().map(|(file, bytes)| (*file, *bytes)))?,
    );
    let images = image_files()?;
    fields.insert(
        "images".into(),
        hashes(
            images
                .iter()
                .map(|(file, bytes)| (file.as_str(), bytes.as_slice())),
        )?,
    );
    let browser =
        json!({ "version": pinned("version")?, "binary_sha256": pinned("binary_sha256")? });
    fields.insert("browser".into(), browser);
    fields.insert(
        "flags".into(),
        json!(render_flags(WIDTH, HEIGHT, f64::from(scale))),
    );
    Ok(fields)
}

/// The page Chromium loads for fixture `name`: the fixture split as the
/// live screenshots split it (`paint_support::chromiums`).
///
/// # Errors
/// When the fixture cannot be read or split.
pub fn page(name: &str) -> Result<String, String> {
    let path = fixtures_dir().join(format!("{name}.wisp"));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(html(&split(name, &text)?))
}

/// The quoted value of `chromium.<field>` under `[downloads]` in
/// `.config/tools.toml`.
///
/// # Errors
/// When the tools file pins no such field.
pub fn pinned(field: &str) -> Result<String, String> {
    let key = format!("chromium.{field}");
    let mut inside = false;
    TOOLS
        .lines()
        .map(str::trim)
        .find_map(|line| {
            if line.starts_with('[') {
                inside = line == "[downloads]";
                return None;
            }
            let (name, value) = line.split_once('=')?;
            (inside && name.trim() == key).then(|| value.trim().trim_matches('"').to_owned())
        })
        .ok_or_else(|| format!(".config/tools.toml pins no [downloads] {key}; fix: add it (cargo xtask setup reads it)"))
}

/// Each file's SHA-256, by name.
fn hashes<'a>(files: impl Iterator<Item = (&'a str, &'a [u8])>) -> Result<Value, String> {
    let mut by_name = Map::new();
    for (file, bytes) in files {
        by_name.insert(file.to_owned(), json!(sha256(bytes)?));
    }
    Ok(Value::Object(by_name))
}

#[cfg(test)]
#[path = "paint_reference_inputs_tests.rs"]
mod tests;
