//! A review's report and the scorecard, as Markdown. The measured
//! items come from facts.json, written by the script; the judged items
//! come from the model, worst first, and say so. The frames are copied
//! beside the report, so its links outlive the next walk.

use std::path::Path;

use serde_json::Value;

use crate::ux_answer::{Judgement, Usage};
use crate::ux_checklist::Kind;
use crate::ux_review::Prepared;

/// What the header says about the run.
pub struct Header<'a> {
    pub name: &'a str,
    pub model: &'a str,
    pub usage: Usage,
    pub cost: f64,
    pub estimate: u64,
    pub prices: &'a str,
}

/// Worst first: fail, concern, cannot-judge, pass; then by severity.
pub fn worst_first(judgements: &mut [&Judgement]) {
    let rank = |v: &str| {
        ["fail", "concern", "cannot-judge", "pass"]
            .iter()
            .position(|r| *r == v)
            .unwrap_or(4)
    };
    judgements.sort_by(|a, b| {
        rank(&a.verdict)
            .cmp(&rank(&b.verdict))
            .then(b.severity.cmp(&a.severity))
    });
}

/// The report's Markdown; frames are linked under `frames_dir`.
pub fn report(
    header: &Header,
    prepared: &Prepared,
    judgements: &[Judgement],
    frames_dir: &str,
) -> String {
    let usage = header.usage;
    let mut parts = vec![
        format!("# UX review {}\n\n", header.name),
        format!(
            "{}: {} input tokens ({} cached; estimated at most {}), {} output tokens. Cost ${:.4} at {}.\n\n",
            header.model,
            usage.input,
            usage.cached,
            header.estimate,
            usage.output,
            header.cost,
            header.prices
        ),
        "## Measured by the walk\n\n".to_owned(),
        format!(
            "| Item | {} |\n|---|{}\n",
            names(prepared).join(" | "),
            "---|".repeat(prepared.viewports.len())
        ),
    ];
    for item in prepared.items.iter().filter(|i| i.kind == Kind::Measured) {
        let values: Vec<String> = prepared
            .viewports
            .iter()
            .map(|v| measured(&v.facts, &item.id))
            .collect();
        parts.push(format!(
            "| {}: {} | {} |\n",
            item.id,
            item.text,
            values.join(" | ")
        ));
    }
    parts.push(format!("\n## Judged by {}\n\n| Verdict | Severity | Item | Frames | Note |\n|---|---|---|---|---|\n", header.model));
    parts.extend(rows(judgements, frames_dir));
    parts.concat()
}

/// The scorecard: the latest report's judgements that are not passes,
/// worst first, linked to its frames.
pub fn scorecard(name: &str, judgements: &[Judgement], frames_dir: &str) -> String {
    let open: Vec<Judgement> = judgements
        .iter()
        .filter(|j| j.verdict != "pass")
        .map(clone)
        .collect();
    let mut parts = vec![format!(
        "# UX scorecard\n\nFrom the review {name} ([report](reports/{name}.md)), worst first.\n\n| Verdict | Severity | Item | Frames | Note |\n|---|---|---|---|---|\n"
    )];
    parts.extend(rows(&open, frames_dir));
    if open.is_empty() {
        parts.push("\nEvery judged item passed.\n".to_owned());
    }
    parts.concat()
}

fn clone(j: &Judgement) -> Judgement {
    Judgement {
        item: j.item.clone(),
        verdict: j.verdict.clone(),
        severity: j.severity,
        frames: j.frames.clone(),
        note: j.note.clone(),
    }
}

fn rows(judgements: &[Judgement], frames_dir: &str) -> Vec<String> {
    let mut sorted: Vec<&Judgement> = judgements.iter().collect();
    worst_first(&mut sorted);
    sorted
        .iter()
        .map(|j| {
            let links: Vec<String> = j
                .frames
                .iter()
                .map(|f| format!("[{f}]({frames_dir}/{f})"))
                .collect();
            format!(
                "| {} | {} | {} | {} | {} |\n",
                j.verdict,
                j.severity,
                j.item,
                links.join(", "),
                j.note.replace('|', "\\|").replace('\n', " ")
            )
        })
        .collect()
}

fn names(prepared: &Prepared) -> Vec<&str> {
    prepared.viewports.iter().map(|v| v.name.as_str()).collect()
}

/// A measured fact as the report shows it: a top-level value as JSON,
/// or per step the length of its list, "not measured" where it is null.
fn measured(facts: &Value, id: &str) -> String {
    if let Some(value) = facts.get(id) {
        return value.to_string();
    }
    let steps = facts["steps"].as_array().map_or(&[][..], Vec::as_slice);
    let counts: Vec<String> = steps
        .iter()
        .map(|s| match &s["facts"][id] {
            Value::Array(list) => list.len().to_string(),
            Value::Null => "not measured".to_owned(),
            other => other.to_string(),
        })
        .collect();
    format!("{} (per step)", counts.join(", "))
}

/// Copy every frame to `dir/<viewport>/<file>`.
///
/// # Errors
/// A frame that cannot be copied, naming it.
pub fn copy_frames(prepared: &Prepared, dir: &Path) -> Result<(), String> {
    for frame in prepared.viewports.iter().flat_map(|v| &v.frames) {
        let to = dir.join(&frame.name);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::copy(&frame.path, &to)
            .map_err(|e| format!("copying {} to {}: {e}", frame.path.display(), to.display()))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "ux_report_tests.rs"]
mod tests;
