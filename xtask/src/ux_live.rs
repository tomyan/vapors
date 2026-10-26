//! A live review, in the order that keeps the spend bounded and the key
//! out of every output: the key from its variable, the day's ceiling,
//! the request, the spend recorded before anything else is judged, the
//! usage held to the estimate, then the report, the scorecard and, with
//! `--pr`, the PR comment, each through the key guard.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use crate::ux_answer::{self, Judgement, Usage};
use crate::ux_config::UxConfig;
use crate::ux_count;
use crate::ux_ledger::{self, Run};
use crate::ux_report::{self, Header};
use crate::ux_review::Prepared;
use crate::ux_secret::Secret;

/// The key from `ux.key_env`.
///
/// # Errors
/// The variable unset or empty, or a key the guard cannot guard.
pub fn key(ux: &UxConfig) -> Result<Secret, String> {
    match std::env::var(&ux.key_env) {
        Ok(key) if !key.is_empty() => Secret::new(key),
        _ => Err(format!(
            "{} is not set, so no review can be sent; fix: run with the API key in {}",
            ux.key_env, ux.key_env
        )),
    }
}

/// Send the review and write what it found; returns the lines to print.
///
/// # Errors
/// Any step's refusal; the spend is recorded first once a response came.
pub fn run(
    ux: &UxConfig,
    prepared: &Prepared,
    secret: &Secret,
    pr: Option<u32>,
) -> Result<Vec<String>, String> {
    let now = ux_ledger::utc(SystemTime::now());
    let ledger = ux.report_dir.join(ux_ledger::FILE);
    let spent = ux_ledger::spent_on(&ledger, &now[..10])?;
    ux_ledger::within_day(
        spent,
        prepared.estimate.worst_cost(&ux.caps, &ux.prices),
        ux.caps.daily_usd,
    )?;
    let sha = head()?;
    let request = request_file(ux, prepared)?;
    let counted = ux_count::exact(ux, secret, &request)?;
    ux_ledger::record_count(&ledger, &now, &sha, ux.model.name, counted)?;
    ux_count::within(ux, prepared.estimate.input(), counted, spent)?;
    let (usage, response) = send(ux, secret, &request)?;
    let run = Run {
        time: &now,
        sha: &sha,
        model: ux.model.name,
        usage,
        cost: usage.cost(&ux.prices),
    };
    let cost = settle(&ledger, counted, &run)?;
    let judgements = ux_answer::judgements(&response, prepared)?;
    let name = format!("{}-{}-{}", &now[..10], &sha[..7], ux.model.name);
    let header = Header {
        name: &name,
        model: ux.model.name,
        usage,
        cost,
        estimate: prepared.estimate.input(),
        prices: &ux.prices.source,
    };
    let report = write_outputs(ux, prepared, secret, &header, &judgements)?;
    if let Some(number) = pr {
        comment(secret, number, &report)?;
    }
    Ok(vec![format!(
        "{name}: {} input and {} output tokens, ${cost:.4}; report {}",
        usage.input,
        usage.output,
        report.display()
    )])
}

/// Record the run's spend, then refuse billed input over the count,
/// which would mean the caps were checked against a number that was not
/// what is billed. Returns the cost.
fn settle(ledger: &Path, counted: u64, run: &Run) -> Result<f64, String> {
    ux_ledger::record(ledger, run)?;
    let used = run.usage.input;
    if used > counted {
        return Err(format!(
            "the request was billed {used} input tokens, over the {counted} the API counted before it was sent; the count is not what is billed; fix: check the count endpoint's answer in work/count-response.json"
        ));
    }
    Ok(run.cost)
}

/// Write the request body, which is counted and then sent unchanged.
fn request_file(ux: &UxConfig, prepared: &Prepared) -> Result<PathBuf, String> {
    let work = ux.report_dir.join("work");
    std::fs::create_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;
    let file = work.join("request.json");
    let text = crate::ux_request::body(ux, prepared)?.to_string();
    std::fs::write(&file, text).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(file)
}

/// Post the request and read its usage.
fn send(
    ux: &UxConfig,
    secret: &Secret,
    request: &Path,
) -> Result<(Usage, serde_json::Value), String> {
    let response = crate::ux_call::post(
        &ux.endpoint,
        secret,
        request,
        &request.with_file_name("response.json"),
        crate::ux_call::DEADLINE,
    )?;
    Ok((ux_answer::usage(&response)?, response))
}

/// The report, its frames and the scorecard, each through the guard.
fn write_outputs(
    ux: &UxConfig,
    prepared: &Prepared,
    secret: &Secret,
    header: &Header,
    judgements: &[Judgement],
) -> Result<PathBuf, String> {
    let reports = ux.report_dir.join("reports");
    ux_report::copy_frames(prepared, &reports.join(header.name))?;
    let report = reports.join(format!("{}.md", header.name));
    let text = ux_report::report(header, prepared, judgements, header.name);
    write(&report, secret.guard("the report", &text)?)?;
    let card = ux_report::scorecard(header.name, judgements, &format!("reports/{}", header.name));
    write(
        &ux.report_dir.join("scorecard.md"),
        secret.guard("the scorecard", &card)?,
    )?;
    Ok(report)
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The commit reviewed.
fn head() -> Result<String, String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .map_err(|e| format!("could not run git: {e}"))?;
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !output.status.success() || sha.len() < 7 {
        return Err(format!(
            "git rev-parse HEAD failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(sha)
}

/// Post the report as a comment on PR `number`, through gh.
fn comment(secret: &Secret, number: u32, report: &Path) -> Result<(), String> {
    let text = std::fs::read_to_string(report).map_err(|e| format!("{}: {e}", report.display()))?;
    secret.guard("the PR comment", &text)?;
    let output = Command::new("gh")
        .args(["pr", "comment", &number.to_string(), "--body-file"])
        .arg(report)
        .output()
        .map_err(|e| format!("could not run gh: {e}; fix: install gh and log in"))?;
    if output.status.success() {
        return Ok(());
    }
    let said = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "gh pr comment {number} failed: {}",
        secret.guard("gh's error", said.trim())?
    ))
}

#[cfg(test)]
#[path = "ux_live_tests.rs"]
mod tests;
