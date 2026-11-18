//! The build script's report of tags whose props were not checked: one
//! summary line, and with `WISP_NOTES=1` each tag's note after it.

use std::collections::BTreeMap;
use std::ffi::OsString;

/// What `WISP_NOTES` asks for: `true` for `1`, `false` when unset or `0`;
/// any other value is refused, naming the variable, value and fix.
pub(super) fn notes_wanted(value: Option<OsString>) -> Result<bool, String> {
    match value.as_deref().and_then(|v| v.to_str()) {
        None if value.is_none() => Ok(false),
        Some("0") => Ok(false),
        Some("1") => Ok(true),
        _ => Err(format!(
            "error: WISP_NOTES=\"{}\" is not understood: use WISP_NOTES=1 for each unchecked tag's position, or 0 or unset for one summary line; rerun with `WISP_NOTES=1 cargo build`",
            value.unwrap_or_default().to_string_lossy()
        )),
    }
}

/// The `cargo::warning` lines for `unchecked` (the compiler's per-tag
/// notes): none when empty, else the summary, then each note if `notes`.
pub(super) fn warning_lines(unchecked: &[String], notes: bool) -> Vec<String> {
    if unchecked.is_empty() {
        return Vec::new();
    }
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for note in unchecked {
        *counts.entry(tag_name(note)).or_default() += 1;
    }
    let names: Vec<String> = counts
        .iter()
        .map(|(name, count)| format!("{name} ({count})"))
        .collect();
    let summary = format!(
        "{} component tags from other crates compile with props unchecked: {} (a known gap until the compiler reads another crate's props); set WISP_NOTES=1 for each tag's position",
        unchecked.len(),
        names.join(", ")
    );
    let each = unchecked.iter().filter(|_| notes).cloned();
    std::iter::once(summary)
        .chain(each)
        .map(|line| format!("cargo::warning={line}"))
        .collect()
}

/// `Scene` from a note holding `` `<Scene>` is not a component ``; a note
/// of any other shape is a compiler bug and stops the build.
fn tag_name(note: &str) -> &str {
    let name = note
        .split_once(">` is not a component")
        .and_then(|(head, _)| head.rsplit_once("`<"))
        .map(|(_, name)| name);
    name.unwrap_or_else(|| panic!("an unchecked-tag note without its tag name: {note}"))
}
