//! The text sent with the frames: what to judge and how to answer, the
//! judged items, the design files, each frame's visible text, and the
//! measured facts. The frames themselves follow, in the order listed.

use crate::ux_checklist::{Item, Kind};
use crate::ux_run::Viewport;

/// What the model is asked to do. Its answer's shape is fixed by the
/// request's JSON schema; this says what goes in it.
const INSTRUCTIONS: &str = "You are reviewing frames from a scripted walkthrough of a web app, taken at each step on each viewport. \
Judge only the judged items listed below, from the frames and their visible text, against the design files. \
For each item give a verdict (pass, concern, fail, or cannot-judge when the frames do not show it), a severity from 1 (minor) to 3 (blocks a person), \
the frames that show it, by the names listed, and a note of at most three sentences saying what a person would see. \
The measured facts are given for context only: they are measured by the script, so do not judge them again.";

/// The prompt for `items`' judged entries over `viewports`, with
/// `design` as (path, text) pairs.
pub fn text(items: &[Item], viewports: &[Viewport], design: &[(String, String)]) -> String {
    let mut parts = vec![format!("{INSTRUCTIONS}\n\n## Judged items\n")];
    let judged = items.iter().filter(|i| i.kind == Kind::Judged);
    parts.extend(judged.map(|item| format!("- {}: {}\n", item.id, item.text)));
    parts.extend(
        design
            .iter()
            .map(|(path, text)| format!("\n## Design file {path}\n{text}\n")),
    );
    parts.push("\n## Frames, in the order they are attached\n".to_owned());
    let frames = viewports.iter().flat_map(|v| &v.frames);
    parts.extend(frames.map(|f| format!("\n### {}\nVisible text:\n{}\n", f.name, f.text)));
    parts.extend(
        viewports
            .iter()
            .map(|v| format!("\n## Measured facts, {}\n{}\n", v.name, v.facts)),
    );
    parts.concat()
}

/// Every measured item names a fact each viewport recorded: a key of its
/// facts.json, or of a step's `facts`.
///
/// # Errors
/// The first measured item a viewport has no fact for, naming both and
/// the checklist line.
pub fn measured_present(
    checklist: &str,
    items: &[Item],
    viewports: &[Viewport],
) -> Result<(), String> {
    for item in items.iter().filter(|i| i.kind == Kind::Measured) {
        for viewport in viewports {
            let steps = viewport.facts["steps"]
                .as_array()
                .map_or(&[][..], Vec::as_slice);
            let found = viewport.facts.get(&item.id).is_some()
                || steps.iter().any(|s| s["facts"].get(&item.id).is_some());
            if !found {
                return Err(format!(
                    "{checklist}:{}: the measured item `{}` names no fact in {}/facts.json; fix: the walk records it, or the checklist names a fact it records",
                    item.line, item.id, viewport.name
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "ux_prompt_tests.rs"]
mod tests;
