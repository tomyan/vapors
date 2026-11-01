//! What `cargo xtask gate` runs, read from the workflow with a YAML
//! parser, so there is one list: every `run` step of every job in file
//! order, local composite actions (`uses: ./path`) expanded into theirs,
//! less the runner's setup (`[gate.setup]`, exact lines). Each step gets
//! the environment CI gives it (the workflow's `env`, the job's, the
//! step's, and `CI=true`) and its job's deadline. A third-party action
//! is not run, and must be listed in `[gate.actions]` with the reason.
//! Anything else the gate does not model is refused naming the job and
//! the step.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use serde_json::Value;

use crate::gate_report::check_timeout_report;
use crate::gate_step::{JOB_KEYS, timeout_of};
use crate::gate_steps::Planner;
use crate::gate_workflow::{WORKFLOW_KEYS, check_keys, env_of, workspace};

/// One step the gate runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedStep {
    /// `job: step name` (or the step's first line when it has no name).
    pub name: String,
    /// The script, run by `bash -eo pipefail -c` as CI runs it.
    pub run: String,
    /// The environment CI gives the step.
    pub env: BTreeMap<String, String>,
    /// Where it runs, relative to the gate's root.
    pub dir: String,
    /// Its job, whose `timeout-minutes` bounds the job's steps together.
    pub job: String,
    /// The job's `timeout-minutes`.
    pub job_timeout: Duration,
    /// The step's own `timeout-minutes`, if any.
    pub step_timeout: Option<Duration>,
}

/// What the plan reads besides the workflow.
pub struct Sources<'a> {
    /// `[gate.setup]`: the runner's setup, exact lines.
    pub setup: &'a BTreeSet<String>,
    /// `[gate.actions]`: third-party actions not run, each with a reason.
    pub actions: &'a BTreeMap<String, String>,
    /// A file's text by its path relative to the gate's root.
    pub read: &'a dyn Fn(&str) -> Result<String, String>,
}

/// The steps to run for `workflow`.
///
/// # Errors
///
/// A workflow that is not YAML, a step the gate does not model (by job
/// and step), or a `[gate.setup]` or `[gate.actions]` entry nothing uses.
pub fn plan(workflow: &str, sources: &Sources<'_>) -> Result<Vec<PlannedStep>, String> {
    let doc = crate::gate_yaml::parse(workflow, "the workflow")?;
    check_keys(&doc, &WORKFLOW_KEYS, "the workflow")?;
    let jobs = doc
        .get("jobs")
        .and_then(Value::as_object)
        .ok_or("no `jobs:`")?;
    let mut planner = Planner::new(sources, workspace(&doc, jobs)?);
    let mut base = BTreeMap::from([("CI".to_owned(), "true".to_owned())]);
    base.extend(env_of(&doc, "the workflow")?);
    let mut steps = Vec::new();
    let mut earlier = BTreeSet::new();
    for (name, job) in jobs {
        let name = name.as_str();
        check_needs(job, name, &earlier)?;
        steps.extend(plan_job(&mut planner, name, job, &base)?);
        earlier.insert(name);
    }
    planner.check_all_used()?;
    Ok(steps)
}

/// A job's `needs:` (a name or a list) must name jobs earlier in the
/// file: the gate runs jobs in file order in one worktree, so an earlier
/// job's work (target/web for the gpu job) is there; a later or unknown
/// one is refused, naming it.
fn check_needs(job: &Value, name: &str, earlier: &BTreeSet<&str>) -> Result<(), String> {
    let needs: Vec<&Value> = match job.get("needs") {
        None => return Ok(()),
        Some(Value::Array(list)) => list.iter().collect(),
        Some(one) => vec![one],
    };
    for need in needs {
        let need = need
            .as_str()
            .ok_or_else(|| format!("{name}: `needs:` is not a job name or a list of them"))?;
        if !earlier.contains(need) {
            return Err(format!(
                "{name}: `needs: {need}`, which is not a job earlier in the file; the gate runs jobs in file order; fix: move {need} before {name}"
            ));
        }
    }
    Ok(())
}

/// The one job condition the gate models: `${{ !cancelled() }}`, which
/// runs a job after a job it needs failed. The gate stops at its first
/// failure and never cancels, so it reaches the job only when every
/// earlier one passed, and runs it then, as CI does.
fn check_job_if(job: &Value, name: &str) -> Result<(), String> {
    match job.get("if").map(Value::as_str) {
        None | Some(Some("${{ !cancelled() }}")) => Ok(()),
        Some(condition) => Err(format!(
            "{name}: `if: {}`, which the gate cannot evaluate; fix: use `if: ${{{{ !cancelled() }}}}` or none",
            condition.unwrap_or("?")
        )),
    }
}

fn plan_job(
    planner: &mut Planner<'_>,
    name: &str,
    job: &Value,
    base: &BTreeMap<String, String>,
) -> Result<Vec<PlannedStep>, String> {
    check_keys(job, &JOB_KEYS, name)?;
    check_job_if(job, name)?;
    let timeout = timeout_of(job, name)?.ok_or_else(|| {
        format!("{name}: no `timeout-minutes:`; fix: give the job one from measured times")
    })?;
    let mut env = base.clone();
    env.extend(env_of(job, name)?);
    let steps = job
        .get("steps")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{name}: no `steps:`"))?;
    check_timeout_report(name, timeout, steps)?;
    planner.job(name, timeout, &env, steps)
}

#[cfg(test)]
#[path = "gate_plan_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "gate_plan_refusal_tests.rs"]
mod refusal_tests;

#[cfg(test)]
#[path = "gate_plan_action_tests.rs"]
mod action_tests;

#[cfg(test)]
#[path = "gate_plan_job_if_tests.rs"]
mod job_if_tests;
