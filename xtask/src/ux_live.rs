//! A live review, in the order that keeps the spend bounded and the key
//! out of every output: the key from its variable; the request counted
//! and booked under the ledger's lock (`ux_spend`); the request; its
//! usage settled before anything else is judged; then the report, the
//! scorecard and, with `--pr`, the PR comment, each through the key
//! guard.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use crate::ux_answer::{self, Judgement};
use crate::ux_config::UxConfig;
use crate::ux_ledger;
use crate::ux_report::{self, Header};
use crate::ux_review::Prepared;
use crate::ux_secret::Secret;
use crate::ux_spend;

/// The key from `ux.key_env`.
///
/// # Errors
/// The variable unset or empty, or a key the guard cannot guard.
pub fn key(ux: &UxConfig) -> Result<Secret, String> {
    match std::env::var(&ux.key_env) {
        Ok(key) if !key.is_empty() => Secret::new(key).map(|s| s.read_from(&ux.key_env)),
        _ => Err(format!(
            "{} is not set, so no review can be sent; fix: run with the API key in {}",
            ux.key_env, ux.key_env
        )),
    }
}

/// Send the review of `sha` and write what it found; returns the lines
/// to print.
///
/// # Errors
/// Any step's refusal. Once booked, the request stays counted at its
/// worst case until its usage settles it (`ux_spend`).
pub fn run(
    ux: &UxConfig,
    prepared: &Prepared,
    secret: &Secret,
    pr: Option<u32>,
    sha: &str,
) -> Result<Vec<String>, String> {
    let request = request_file(ux, prepared)?;
    let (booked, counted) = ux_spend::book_review(ux, prepared, secret, &request, sha)?;
    let response = crate::ux_call::post(
        &ux.endpoint,
        secret,
        &request,
        &request.with_file_name("response.json"),
        crate::ux_call::DEADLINE,
    )?;
    let usage = ux_answer::usage(&response)?;
    let cost = ux_spend::settle(ux, &booked, usage, counted)?;
    let judgements = ux_answer::judgements(&response, prepared)?;
    let date = &ux_ledger::utc(SystemTime::now())[..10];
    let name = format!("{date}-{}-{}", &sha[..7.min(sha.len())], ux.model.name);
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

/// Write the request body, which is counted and then sent unchanged.
fn request_file(ux: &UxConfig, prepared: &Prepared) -> Result<PathBuf, String> {
    let work = ux.report_dir.join("work");
    std::fs::create_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;
    let file = work.join("request.json");
    let text = crate::ux_request::body(ux, prepared)?.to_string();
    std::fs::write(&file, text).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(file)
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

/// The commit reviewed: the working directory's HEAD.
pub fn head(secret: &Secret) -> Result<String, String> {
    let mut git = Command::new("git");
    git.args(["rev-parse", "HEAD"])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    secret.unset_in(&mut git);
    let output = git
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
    let mut gh = Command::new("gh");
    gh.args(["pr", "comment", &number.to_string(), "--body-file"])
        .arg(report);
    secret.unset_in(&mut gh);
    let output = gh
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
#[path = "ux_live_spend_tests.rs"]
mod spend_tests;
#[cfg(test)]
#[path = "ux_live_tests.rs"]
mod tests;
