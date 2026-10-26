//! `cargo xtask ux-review`: the walk's frames and facts, the design files
//! and the judged items of the UX checklist, sent to a model for the
//! items no script can measure (docs/ux-signal.md in touche). Settings
//! are `[ux]` in factory.toml. `--dry-run` builds the request and prints
//! its estimated tokens and worst-case cost, and sends nothing.

use std::process::ExitCode;

use crate::config::Config;
use crate::ux_checklist::{self, Item, Kind};
use crate::ux_config::UxConfig;
use crate::ux_estimate::{Estimate, image_tokens, text_tokens};
use crate::ux_run::{self, Viewport};

const REPRODUCE: &str = "cargo xtask ux-review --dry-run";

/// Run the review; `dry_run` stops before anything is sent.
pub fn run(dry_run: bool) -> ExitCode {
    match review(dry_run) {
        Ok(lines) => {
            for line in lines {
                println!("ok   ux-review: {line}");
            }
            ExitCode::SUCCESS
        }
        Err(why) => {
            println!("FAIL ux-review: {why}");
            println!("     reproduce: {REPRODUCE}");
            ExitCode::FAILURE
        }
    }
}

/// What a review sends, read and checked.
pub struct Prepared {
    pub items: Vec<Item>,
    pub viewports: Vec<Viewport>,
    pub prompt: String,
    pub estimate: Estimate,
}

fn review(dry_run: bool) -> Result<Vec<String>, String> {
    let config = Config::load().map_err(|e| e.to_string())?;
    let ux = UxConfig::from_config(&config, std::env::var("HOME").ok().as_deref())?;
    let prepared = prepare(&ux)?;
    checked(&ux, &prepared, dry_run)
}

/// The caps checked and what would be sent, summarised; the live call
/// is refused, naming the flag, until slice 2 builds it.
///
/// # Errors
/// A cap exceeded, or no `--dry-run`.
pub fn checked(ux: &UxConfig, prepared: &Prepared, dry_run: bool) -> Result<Vec<String>, String> {
    prepared.estimate.within(&ux.caps, &ux.prices)?;
    if !dry_run {
        return Err(
            "the live review is not built yet (U4 slice 2); fix: pass --dry-run".to_owned(),
        );
    }
    let mut lines = summary(ux, prepared);
    lines.push("dry run: nothing was sent".to_owned());
    Ok(lines)
}

/// Read the checklist, the run and the design files, and build the
/// prompt and its estimate.
///
/// # Errors
/// Any of them unreadable or of a shape `ux_checklist`, `ux_run` or
/// `ux_prompt` refuses.
pub fn prepare(ux: &UxConfig) -> Result<Prepared, String> {
    let checklist = ux.checklist.display().to_string();
    let text = std::fs::read_to_string(&ux.checklist).map_err(|e| format!("{checklist}: {e}"))?;
    let items = ux_checklist::parse(&checklist, &text)?;
    let viewports = ux_run::read(&ux.run_dir)?;
    crate::ux_prompt::measured_present(&checklist, &items, &viewports)?;
    let design = ux
        .design
        .iter()
        .map(|path| {
            std::fs::read_to_string(path)
                .map(|text| (path.display().to_string(), text))
                .map_err(|e| format!("{}: {e}", path.display()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let prompt = crate::ux_prompt::text(&items, &viewports, &design);
    let images = viewports
        .iter()
        .flat_map(|v| &v.frames)
        .map(|f| image_tokens(ux.model, f.size, &ux.image_detail))
        .sum();
    let estimate = Estimate {
        text: text_tokens(prompt.len()),
        images,
    };
    Ok(Prepared {
        items,
        viewports,
        prompt,
        estimate,
    })
}

fn summary(ux: &UxConfig, prepared: &Prepared) -> Vec<String> {
    let frames: usize = prepared.viewports.iter().map(|v| v.frames.len()).sum();
    let judged = prepared
        .items
        .iter()
        .filter(|i| i.kind == Kind::Judged)
        .count();
    let estimate = &prepared.estimate;
    vec![
        format!(
            "{} viewports, {frames} frames, {judged} judged and {} measured items",
            prepared.viewports.len(),
            prepared.items.len() - judged
        ),
        format!(
            "about {} input tokens ({} text from a {}-byte prompt, {} images) of {}; output capped at {}",
            estimate.input(),
            estimate.text,
            prepared.prompt.len(),
            estimate.images,
            ux.caps.input_tokens,
            ux.caps.output_tokens
        ),
        format!(
            "worst case ${:.4} of ${:.2} a run, {} at ${}/${} per million ({})",
            estimate.worst_cost(&ux.caps, &ux.prices),
            ux.caps.cost_usd,
            ux.model.name,
            ux.prices.input,
            ux.prices.output,
            ux.prices.source
        ),
    ]
}

#[cfg(test)]
#[path = "ux_review_tests.rs"]
mod tests;
