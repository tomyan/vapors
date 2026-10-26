//! `cargo xtask check-docs [--offline]`: the docs say what the code and
//! GitHub say. Each rule is set in `[docs]`, or excused under
//! `[docs.not_applying]` with its reason; one neither set nor excused is
//! a failure, so no rule is skipped unsaid.
//! - `claude`: the commands CLAUDE.md names exist (`docs_commands.rs`);
//!   `app` names the package whose `--help` lists the CLI's commands.
//! - `plan` with `repo`: each work item's State names its PRs as gh sees
//!   them (`docs_plan.rs`). `--offline` skips only the comparison with gh,
//!   and says so; a State in no shape still fails.
//! - `patterns` with `page`: "double-quoted" text in the patterns file is
//!   in the page's source under `page`'s directories (`docs_patterns.rs`).
//!
//! Each failure names the file and line, and the fix. Declared limit: a
//! documented command's flags and arguments are not checked, only its name.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use crate::config::Config;
use crate::docs_commands::{App, check, commands_in_help};
use crate::docs_plan::{Claim, Pr, contradiction, label, read_plan};
use crate::docs_sources::{app_help, has_example, page_sources, pr, xtask_commands};

/// The rules, by their `[docs]` key.
const RULES: [&str; 3] = ["claude", "plan", "patterns"];

/// Check the docs; with `offline`, PR states are not asked of gh.
pub fn run(offline: bool) -> ExitCode {
    let outcome = Config::load()
        .map_err(|e| vec![e.to_string()])
        .and_then(|config| rules(&config, offline));
    match outcome {
        Ok(lines) => {
            for line in lines {
                println!("ok   docs: {line}");
            }
            ExitCode::SUCCESS
        }
        Err(problems) => {
            for problem in problems {
                println!("FAIL docs: {problem}");
            }
            println!("     reproduce: cargo xtask check-docs");
            ExitCode::FAILURE
        }
    }
}

fn rules(config: &Config, offline: bool) -> Result<Vec<String>, Vec<String>> {
    let excused = config.entries_under("docs.not_applying.");
    let (mut said, mut problems) = (Vec::new(), Vec::new());
    for rule in RULES {
        let outcome = match (config.get(&format!("docs.{rule}")), excused.get(rule)) {
            (Some(file), _) => run_rule(rule, file, config, offline),
            (None, Some(reason)) => Ok(vec![format!("{rule}: does not apply here: {reason}")]),
            (None, None) => Err(vec![format!(
                "[docs] {rule} is not set and [docs.not_applying] gives no reason; fix: set one in .config/factory.toml"
            )]),
        };
        match outcome {
            Ok(lines) => said.extend(lines),
            Err(found) => problems.extend(found),
        }
    }
    if problems.is_empty() {
        Ok(said)
    } else {
        Err(problems)
    }
}

fn run_rule(
    rule: &str,
    file: &str,
    config: &Config,
    offline: bool,
) -> Result<Vec<String>, Vec<String>> {
    let text = std::fs::read_to_string(file).map_err(|e| vec![format!("{file}: {e}")])?;
    match rule {
        "claude" => commands(file, &text, config.get("docs.app")),
        "plan" => plan(file, &text, config.get("docs.repo"), offline),
        "patterns" => patterns(file, &text, config.get("docs.page")),
        _ => Err(vec![format!(
            "[docs] {rule}: this rule is not built yet; fix: excuse it under [docs.not_applying]"
        )]),
    }
}

fn patterns(file: &str, text: &str, page: Option<&str>) -> Result<Vec<String>, Vec<String>> {
    let page = page.ok_or_else(|| {
        vec!["[docs] patterns needs [docs] page (the page's source directories, space-separated); fix: set it".to_owned()]
    })?;
    let sources = page_sources(Path::new("."), page).map_err(|e| vec![e])?;
    let problems = crate::docs_patterns::check(file, text, &sources, page);
    if problems.is_empty() {
        Ok(vec![format!(
            "patterns: every quote in {file} is in {page}"
        )])
    } else {
        Err(problems)
    }
}

fn commands(file: &str, text: &str, app: Option<&str>) -> Result<Vec<String>, Vec<String>> {
    let app = match app {
        Some(package) => {
            let help = app_help(package).map_err(|e| vec![e])?;
            Some(App {
                package: package.to_owned(),
                commands: commands_in_help(&help).map_err(|e| vec![e])?,
            })
        }
        None => None,
    };
    let xtask = xtask_commands();
    let names: Vec<&str> = xtask.iter().map(String::as_str).collect();
    let examples = |package: &str, example: &str| has_example(Path::new("."), package, example);
    let problems = check(file, text, &names, app.as_ref(), &examples);
    if problems.is_empty() {
        Ok(vec![format!("claude: every command {file} names exists")])
    } else {
        Err(problems)
    }
}

/// Each claim against gh, a PR in another repository looked up under
/// `repo`'s owner; each PR asked once.
fn compare(file: &str, claims: &[Claim], repo: &str) -> Vec<String> {
    let owner = repo.split_once('/').map_or(repo, |(owner, _)| owner);
    let mut seen: BTreeMap<(String, u64), Result<Pr, String>> = BTreeMap::new();
    let mut problems = Vec::new();
    for claim in claims {
        let full = claim
            .repo
            .as_ref()
            .map_or_else(|| repo.to_owned(), |name| format!("{owner}/{name}"));
        match seen
            .entry((full.clone(), claim.pr))
            .or_insert_with(|| pr(&full, claim.pr))
        {
            Ok(found) => problems.extend(contradiction(file, claim, found)),
            Err(why) => problems.push(format!("{file}:{}: {}: {why}", claim.line, label(claim))),
        }
    }
    problems
}

fn plan(
    file: &str,
    text: &str,
    repo: Option<&str>,
    offline: bool,
) -> Result<Vec<String>, Vec<String>> {
    let (claims, mut problems) = read_plan(file, text);
    if offline {
        return if problems.is_empty() {
            Ok(vec![format!(
                "plan: {} PR states read; not compared with gh: --offline",
                claims.len()
            )])
        } else {
            Err(problems)
        };
    }
    let repo = repo.ok_or_else(|| {
        vec!["[docs] plan needs [docs] repo (owner/name); fix: set it".to_owned()]
    })?;
    problems.extend(compare(file, &claims, repo));
    if problems.is_empty() {
        Ok(vec![format!(
            "plan: {} PR states agree with gh",
            claims.len()
        )])
    } else {
        Err(problems)
    }
}

#[cfg(test)]
#[path = "docs_rules_tests.rs"]
mod tests;
