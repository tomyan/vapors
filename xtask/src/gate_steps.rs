//! Planning a job's steps: a run step becomes a planned step unless it is
//! all setup; a local composite action's steps are planned in its place,
//! run from the workspace root as CI runs them; a third-party action is
//! skipped when `[gate.actions]` lists it. Records which setup lines and
//! listed actions were used, so a stale entry is refused.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use serde_yaml::Value;

use crate::gate_action::composite;
use crate::gate_plan::{PlannedStep, Sources};
use crate::gate_step::{
    ACTION_STEP_KEYS, WORKFLOW_STEP_KEYS, label, lines, runs_on_success, script, timeout_of,
};
use crate::gate_workflow::{check_keys, env_of};

/// What a step inherits: its job, and the action it is part of, if any.
struct Scope<'a> {
    job: &'a str,
    /// What step labels start with: the job, or the calling step's label.
    prefix: &'a str,
    timeout: Duration,
    env: &'a BTreeMap<String, String>,
    step_timeout: Option<Duration>,
    /// The inputs of the composite action being expanded, if any.
    inputs: Option<&'a BTreeMap<String, String>>,
}

impl Scope<'_> {
    fn planned(&self, label: &str, run: &str, dir: String) -> PlannedStep {
        PlannedStep {
            name: label.to_owned(),
            run: run.trim_end().to_owned(),
            env: self.env.clone(),
            dir,
            job: self.job.to_owned(),
            job_timeout: self.timeout,
            step_timeout: self.step_timeout,
        }
    }
}

/// Plans steps, remembering which setup lines and actions it used.
pub struct Planner<'a> {
    sources: &'a Sources<'a>,
    workspace: String,
    used_setup: BTreeSet<String>,
    used_actions: BTreeSet<String>,
}

impl<'a> Planner<'a> {
    pub fn new(sources: &'a Sources<'a>, workspace: String) -> Self {
        Self {
            sources,
            workspace,
            used_setup: BTreeSet::new(),
            used_actions: BTreeSet::new(),
        }
    }

    /// The planned steps of job `name`.
    pub fn job(
        &mut self,
        name: &str,
        timeout: Duration,
        env: &BTreeMap<String, String>,
        steps: &[Value],
    ) -> Result<Vec<PlannedStep>, String> {
        let scope = Scope {
            job: name,
            prefix: name,
            timeout,
            env,
            step_timeout: None,
            inputs: None,
        };
        self.steps(&scope, steps)
    }

    fn steps(&mut self, scope: &Scope<'_>, steps: &[Value]) -> Result<Vec<PlannedStep>, String> {
        let mut planned = Vec::new();
        for step in steps {
            planned.extend(self.step(scope, step)?);
        }
        Ok(planned)
    }

    fn step(&mut self, scope: &Scope<'_>, step: &Value) -> Result<Vec<PlannedStep>, String> {
        let label = label(scope.prefix, step);
        let keys: &[&str] = if scope.inputs.is_some() {
            &ACTION_STEP_KEYS
        } else {
            &WORKFLOW_STEP_KEYS
        };
        check_keys(step, keys, &label)?;
        let runs = runs_on_success(&label, step)?;
        let timeout = timeout_of(step, &label)?.or(scope.step_timeout);
        let mut env = scope.env.clone();
        env.extend(env_of(step, &label)?);
        let inner = Scope {
            env: &env,
            step_timeout: timeout,
            ..*scope
        };
        match step.get("uses").and_then(Value::as_str) {
            Some(uses) => self.uses(&inner, &label, uses, step, runs),
            None => Ok(self.run(&inner, &label, step, runs)?.into_iter().collect()),
        }
    }

    fn run(
        &mut self,
        scope: &Scope<'_>,
        label: &str,
        step: &Value,
        runs: bool,
    ) -> Result<Option<PlannedStep>, String> {
        let run = script(scope.inputs, label, step)?;
        if !runs || self.all_setup(&lines(&run)) {
            return Ok(None);
        }
        let dir = match scope.inputs {
            Some(_) => self.workspace.clone(),
            None => ".".to_owned(),
        };
        Ok(Some(scope.planned(label, &run, dir)))
    }

    fn all_setup(&mut self, lines: &[&str]) -> bool {
        let all = lines.iter().all(|line| self.sources.setup.contains(*line));
        if all {
            self.used_setup
                .extend(lines.iter().map(|line| (*line).to_owned()));
        }
        all
    }

    fn uses(
        &mut self,
        scope: &Scope<'_>,
        label: &str,
        uses: &str,
        step: &Value,
        runs: bool,
    ) -> Result<Vec<PlannedStep>, String> {
        if !uses.starts_with("./") {
            return self.third_party(label, uses).map(|()| Vec::new());
        }
        let path = format!(
            "{}/{}/action.yml",
            self.workspace,
            uses.trim_start_matches("./").trim_end_matches('/')
        );
        let action = composite(&(self.sources.read)(&path)?, &path, step.get("with"))?;
        if !runs {
            return Ok(Vec::new());
        }
        let inner = Scope {
            inputs: Some(&action.inputs),
            prefix: label,
            ..*scope
        };
        self.steps(&inner, &action.steps)
    }

    fn third_party(&mut self, label: &str, uses: &str) -> Result<(), String> {
        let name = uses.split('@').next().unwrap_or(uses);
        if !self.sources.actions.contains_key(name) {
            return Err(format!(
                "{label}: `{uses}`, an action the gate does not run; fix: list \"{name}\" in [gate.actions] in .config/factory.toml with the reason it need not run locally"
            ));
        }
        self.used_actions.insert(name.to_owned());
        Ok(())
    }

    /// Refuses a `[gate.setup]` line or `[gate.actions]` entry no step used:
    /// a stale entry would skip a step added later that matches it.
    pub fn check_all_used(&self) -> Result<(), String> {
        if let Some(stale) = self
            .sources
            .setup
            .iter()
            .find(|line| !self.used_setup.contains(*line))
        {
            return Err(format!(
                "[gate.setup] `{stale}` makes no step of ci.yml all setup, so it skips nothing; fix: remove it from .config/factory.toml"
            ));
        }
        match self
            .sources
            .actions
            .keys()
            .find(|name| !self.used_actions.contains(*name))
        {
            Some(stale) => Err(format!(
                "[gate.actions] `{stale}` is used by no step in ci.yml; fix: remove it from .config/factory.toml"
            )),
            None => Ok(()),
        }
    }
}
