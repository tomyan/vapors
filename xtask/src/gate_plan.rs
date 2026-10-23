//! What `cargo xtask gate` runs, read from the workflow itself with a
//! YAML parser, so there is one list: every `run` step of every job, in
//! file order, less the runner's setup (`[gate.setup]`, exact lines),
//! each with the environment CI gives it (the workflow's `env`, then the
//! job's, then the step's, and `CI=true`). Anything the gate does not
//! model is refused naming the job and the step, never run some other
//! way than CI runs it.

use std::collections::{BTreeMap, BTreeSet};

use serde_yaml::Value;

use crate::gate_workflow::{check_defaults, check_setup_used, env_of};

/// One step the gate runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedStep {
    /// `job: step name` (or the step's first line when it has no name).
    pub name: String,
    /// The `run` script, run by `bash -eo pipefail -c` as CI runs it.
    pub run: String,
    /// The environment CI gives the step.
    pub env: BTreeMap<String, String>,
}

/// The steps to run for `workflow`, given the runner's `setup` lines.
///
/// # Errors
///
/// A workflow that is not YAML, or a step the gate does not model, by
/// job and step.
pub fn plan(workflow: &str, setup: &BTreeSet<String>) -> Result<Vec<PlannedStep>, String> {
    let doc: Value = serde_yaml::from_str(workflow).map_err(|e| format!("not YAML: {e}"))?;
    check_defaults(&doc)?;
    check_setup_used(&doc, setup)?;
    let mut base = BTreeMap::from([("CI".to_owned(), "true".to_owned())]);
    base.extend(env_of(&doc, "the workflow")?);
    let jobs = doc
        .get("jobs")
        .and_then(Value::as_mapping)
        .ok_or("no `jobs:`")?;
    let mut planned = Vec::new();
    for (name, job) in jobs {
        let name = name.as_str().ok_or("a job name that is not a string")?;
        planned.extend(plan_job(name, job, &base, setup)?);
    }
    Ok(planned)
}

fn plan_job(
    job: &str,
    value: &Value,
    base: &BTreeMap<String, String>,
    setup: &BTreeSet<String>,
) -> Result<Vec<PlannedStep>, String> {
    if value
        .get("strategy")
        .and_then(|s| s.get("matrix"))
        .is_some()
    {
        return Err(format!(
            "{job}: a matrix, which the gate does not model; fix: run its cases as steps"
        ));
    }
    let mut env = base.clone();
    env.extend(env_of(value, job)?);
    let steps = value
        .get("steps")
        .and_then(Value::as_sequence)
        .ok_or_else(|| format!("{job}: no `steps:`"))?;
    let mut planned = Vec::new();
    for step in steps {
        planned.extend(plan_step(job, step, &env, setup)?);
    }
    Ok(planned)
}

fn plan_step(
    job: &str,
    step: &Value,
    env: &BTreeMap<String, String>,
    setup: &BTreeSet<String>,
) -> Result<Option<PlannedStep>, String> {
    let label = label(job, step);
    if step.get("uses").is_some() {
        return uses_step(&label, step).map(|()| None);
    }
    let run = step
        .get("run")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{label}: `run` is not a string"))?;
    refuse_unmodelled(&label, step, run)?;
    if !runs_on_success(&label, step)? {
        return Ok(None);
    }
    let lines: Vec<&str> = run
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.iter().all(|line| setup.contains(*line)) {
        return Ok(None);
    }
    let mut env = env.clone();
    env.extend(env_of(step, &label)?);
    Ok(Some(PlannedStep {
        name: label,
        run: run.trim_end().to_owned(),
        env,
    }))
}

/// A `uses` step is not run; its `if:` must be one CI's success path skips.
fn uses_step(label: &str, step: &Value) -> Result<(), String> {
    match step.get("if").map(Value::as_str) {
        None | Some(Some("failure()" | "always()")) => Ok(()),
        Some(condition) => Err(format!(
            "{label}: `if: {}`, which the gate does not model; fix: use failure() or always()",
            condition.unwrap_or("?")
        )),
    }
}

/// Whether CI runs a run step when every step before it passed: with no
/// `if:`, `success()` or `always()` it does; with `failure()` it does not,
/// and the gate stops at its first failure. Any other condition is refused.
fn runs_on_success(label: &str, step: &Value) -> Result<bool, String> {
    match step.get("if").map(Value::as_str) {
        None | Some(Some("success()" | "always()")) => Ok(true),
        Some(Some("failure()")) => Ok(false),
        Some(condition) => Err(format!(
            "{label}: `if: {}` on a run step, which the gate cannot evaluate; fix: use success(), always() or failure()",
            condition.unwrap_or("?")
        )),
    }
}

fn refuse_unmodelled(label: &str, step: &Value, run: &str) -> Result<(), String> {
    for key in ["shell", "working-directory"] {
        if step.get(key).is_some() {
            return Err(format!(
                "{label}: `{key}:`, which the gate does not model; fix: drop it or teach the gate"
            ));
        }
    }
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
    Ok(())
}

/// `job: name`, or `job: uses`, or `job: <first line of run>`.
fn label(job: &str, step: &Value) -> String {
    let text = |key: &str| step.get(key).and_then(Value::as_str);
    let first_line = text("run").and_then(|run| run.lines().find(|line| !line.trim().is_empty()));
    let what = text("name")
        .or_else(|| text("uses"))
        .or(first_line)
        .unwrap_or("a step");
    format!("{job}: {}", what.trim())
}

#[cfg(test)]
#[path = "gate_plan_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "gate_plan_refusal_tests.rs"]
mod refusal_tests;
