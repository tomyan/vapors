//! The gate's rules for one step: which keys it models, whether CI runs
//! the step when every step before it passed, what a `run` may hold, its
//! `timeout-minutes`, and its label.

use std::collections::BTreeMap;
use std::time::Duration;

use serde_json::Value;

use crate::gate_action::substitute;

/// The keys of a workflow step the gate models.
pub const WORKFLOW_STEP_KEYS: [&str; 8] = [
    "name",
    "id",
    "uses",
    "with",
    "run",
    "env",
    "if",
    "timeout-minutes",
];

/// The keys of a composite action's step the gate models; `shell:` is
/// required there and must be `bash`.
pub const ACTION_STEP_KEYS: [&str; 8] = ["name", "id", "uses", "with", "run", "env", "if", "shell"];

/// The keys of a job the gate models.
pub const JOB_KEYS: [&str; 6] = [
    "name",
    "runs-on",
    "timeout-minutes",
    "env",
    "defaults",
    "steps",
];

/// Whether CI runs the step when every step before it passed: with no
/// `if:`, `success()` or `always()` it does; with `failure()` it does not,
/// and the gate stops at its first failure; with `cancelled()` it does
/// not, and the gate never cancels. Any other condition is refused.
pub fn runs_on_success(label: &str, step: &Value) -> Result<bool, String> {
    match step.get("if").map(Value::as_str) {
        None | Some(Some("success()" | "always()")) => Ok(true),
        Some(Some("failure()" | "cancelled()")) => Ok(false),
        Some(condition) => Err(format!(
            "{label}: `if: {}`, which the gate cannot evaluate; fix: use success(), always(), failure() or cancelled()",
            condition.unwrap_or("?")
        )),
    }
}

/// Refuses a script the gate cannot run as CI does.
pub fn check_script(label: &str, run: &str) -> Result<(), String> {
    if run.contains("${{") {
        return Err(format!(
            "{label}: an expression (`${{{{ }}}}`) in `run`, which the gate cannot evaluate"
        ));
    }
    if run.lines().any(|line| line.trim_end().ends_with('\\')) {
        return Err(format!(
            "{label}: a `\\` continuation, which the gate does not model; fix: one command per line"
        ));
    }
    for file in ["GITHUB_ENV", "GITHUB_PATH"] {
        if run.contains(file) {
            return Err(format!(
                "{label}: writes {file}, which changes later steps in a way the gate does not model"
            ));
        }
    }
    Ok(())
}

/// The script's non-empty lines, trimmed.
#[must_use]
pub fn lines(run: &str) -> Vec<&str> {
    run.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
}

/// `timeout-minutes` of `value`, which must be a positive whole number
/// when present.
pub fn timeout_of(value: &Value, label: &str) -> Result<Option<Duration>, String> {
    let Some(minutes) = value.get("timeout-minutes") else {
        return Ok(None);
    };
    match minutes.as_u64() {
        Some(minutes) if minutes > 0 => Ok(Some(Duration::from_secs(minutes * 60))),
        _ => Err(format!(
            "{label}: `timeout-minutes: {minutes:?}` is not a positive whole number"
        )),
    }
}

/// `prefix: name`, or `uses`, or `id`, or the first line of `run`; the
/// prefix is the job, or the calling step's label inside an action.
#[must_use]
pub fn label(prefix: &str, step: &Value) -> String {
    let text = |key: &str| step.get(key).and_then(Value::as_str);
    let first_line = text("run").and_then(|run| run.lines().find(|line| !line.trim().is_empty()));
    let what = text("name")
        .or_else(|| text("uses"))
        .or_else(|| text("id"))
        .or(first_line)
        .unwrap_or("a step");
    format!("{prefix}: {}", what.trim())
}

/// The step's script: in a composite action, with `shell: bash` and its
/// inputs substituted; checked either way.
pub fn script(
    inputs: Option<&BTreeMap<String, String>>,
    label: &str,
    step: &Value,
) -> Result<String, String> {
    let run = step
        .get("run")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{label}: `run` is not a string"))?;
    let run = match inputs {
        Some(inputs) => {
            check_shell(label, step)?;
            substitute(run, inputs, label)?
        }
        None => run.to_owned(),
    };
    check_script(label, &run)?;
    Ok(run)
}

/// A composite action's run step names its shell; the gate models bash.
fn check_shell(label: &str, step: &Value) -> Result<(), String> {
    match step.get("shell").and_then(Value::as_str) {
        Some("bash") => Ok(()),
        other => Err(format!(
            "{label}: `shell: {}`, which the gate does not model; fix: use bash",
            other.unwrap_or("?")
        )),
    }
}
