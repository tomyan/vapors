//! `cargo xtask api-build <name> <brief> <budget>` (`docs/plans/api-builder.md`):
//! one slice built by a headless session billed to the API. It makes the
//! builder's clone, starts the harness server with the key and budget,
//! proves the fences, runs the session in the sandbox, and prints what
//! it spent. The clone is left for the push (slice 4) and for reading.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use crate::api_budget::Budget;
use crate::api_fences_run::{home, host, proven_on, sandboxed};
use crate::api_paths::{Builder, KEY_FILE};
use crate::api_sandbox::BIN;
use crate::api_server::{Proxy, Routes};
use crate::api_session::Ended;
use crate::ux_secret::Secret;

/// One run's inputs.
pub struct Order<'a> {
    pub name: &'a str,
    pub brief: &'a Path,
    pub budget: f64,
    pub model: &'a str,
}

/// Runs `order`.
pub fn run(order: &Order<'_>) -> ExitCode {
    match ran(order) {
        Ok((lines, ended)) => {
            for line in lines {
                println!("{line}");
            }
            if ended == Ended::Exited(Some(0)) {
                return ExitCode::SUCCESS;
            }
            println!("FAIL api-build: the session did not end with status 0: {ended:?}");
            ExitCode::FAILURE
        }
        Err(why) => {
            println!("FAIL api-build: {why}");
            ExitCode::FAILURE
        }
    }
}

fn ran(order: &Order<'_>) -> Result<(Vec<String>, Ended), String> {
    let config = crate::config::Config::load().map_err(|e| e.to_string())?;
    let budget = Budget::new(order.budget, crate::api_prices::prices(&config)?);
    budget.admit(order.model)?;
    let builder = Builder::new(order.name, &home()?)?;
    let secret = key(&builder.home.join(KEY_FILE))?;
    let (main, objects) = crate::api_clone::main_repository()?;
    crate::api_clone::make(&builder, &main, &format!("api/{}", builder.name))?;
    let staging = private(&builder.root().join("requests"))?;
    let routes = Arc::new(Routes::with(Proxy {
        secret,
        budget,
        staging,
    }));
    crate::api_server::serve(&builder.socket(), Arc::clone(&routes))?;
    let host = host(objects)?;
    let mut lines = proven_on(&builder, &host)?;
    let (said, ended) = session(order, &builder, &host, &routes)?;
    lines.extend(said);
    Ok((lines, ended))
}

/// The session run, and the lines that say how it ended.
fn session(
    order: &Order<'_>,
    builder: &Builder,
    host: &crate::api_sandbox::Host,
    routes: &Routes,
) -> Result<(Vec<String>, Ended), String> {
    let claude = Path::new(BIN).join("claude").display().to_string();
    let mut command = vec![claude];
    command.extend(crate::api_session::arguments(order.model, order.budget));
    let words: Vec<&str> = command.iter().map(String::as_str).collect();
    let result = builder.root().join("result.json");
    let proxy = routes.proxy.as_ref().ok_or("the run has no proxy")?;
    let ended = crate::api_session::run(
        sandboxed(builder, host, &words),
        (order.brief, &result),
        &proxy.budget,
    )?;
    let lines = summary(&ended, (&result, builder), routes, proxy)?;
    Ok((lines, ended))
}

fn summary(
    ended: &Ended,
    (result, builder): (&Path, &Builder),
    routes: &Routes,
    proxy: &Proxy,
) -> Result<Vec<String>, String> {
    let (dollars, unknown) = proxy.budget.spent();
    let refused = routes
        .refused
        .lock()
        .map(|log| log.join("; "))
        .unwrap_or_default();
    let unknown = unknown
        .map(|why| format!(" (then unknown: {why})"))
        .unwrap_or_default();
    let lines = vec![
        format!("session: {ended:?}"),
        format!("spent: ${dollars:.4} counted by the proxy{unknown}"),
        format!("refused: {refused}"),
        format!("result: {}", result.display()),
        format!(
            "clone: {} (branch api/{})",
            builder.clone_dir().display(),
            builder.name
        ),
    ];
    agreed(dollars, reported_cost(result))?;
    Ok(lines)
}

/// The proxy's count and the CLI's report held together: two answers to
/// what the session spent. Within a hundredth of a cent, as both price
/// the same tokens; a report the CLI did not write is not compared (a
/// session stopped by the harness writes none).
fn agreed(proxy: f64, cli: Option<f64>) -> Result<(), String> {
    match cli {
        Some(cli) if (proxy - cli).abs() > 0.0001 => Err(format!(
            "the proxy counted ${proxy:.4} and the CLI reported ${cli:.4}: one of them prices \
             the tokens wrongly; compare [api_build.prices] with the CLI's modelUsage in the result"
        )),
        _ => Ok(()),
    }
}

/// The cost the CLI's JSON result reports, if it holds one.
fn reported_cost(result: &Path) -> Option<f64> {
    let text = fs::read_to_string(result).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let entries = value.as_array().cloned().unwrap_or_else(|| vec![value]);
    entries.iter().find(|entry| entry["type"] == "result")?["total_cost_usd"].as_f64()
}

/// The key in `file`, which must be mode 600.
fn key(file: &Path) -> Result<Secret, String> {
    let mode = fs::metadata(file)
        .map_err(|e| format!("the key file {}: {e}", file.display()))?
        .permissions()
        .mode()
        & 0o777;
    if mode != 0o600 {
        return Err(format!(
            "the key file {} is mode {mode:o}; fix: chmod 600 {}",
            file.display(),
            file.display()
        ));
    }
    let key = fs::read_to_string(file)
        .map_err(|e| format!("reading the key file {}: {e}", file.display()))?;
    Secret::new(key.trim().to_owned())
}

/// `dir` made, private to this user.
fn private(dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| format!("making {}: {e}", dir.display()))?;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
        .map_err(|e| format!("making {} private: {e}", dir.display()))?;
    Ok(dir.to_owned())
}
