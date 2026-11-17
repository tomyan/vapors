//! `cargo xtask api-build <name> <brief> <budget>` (`docs/plans/api-builder.md`):
//! one slice built by a headless session billed to the API. It makes the
//! builder's clone, starts the harness server with the key and budget,
//! proves the fences, runs the session in the sandbox, and prints what
//! it spent (`api_build_report.rs`). The clone is left for the push
//! (slice 4), which never runs anything in it (the plan's "The clone is
//! untrusted").

use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

use crate::api_budget::Budget;
use crate::api_fences_run::{proven, sandboxed};
use crate::api_paths::{Builder, KEY_FILE};
use crate::api_sandbox::{BIN, Host};
use crate::api_server::Routes;
use crate::api_server_proxy::Proxy;
use crate::api_session::Ended;

/// One run's inputs.
pub struct Order<'a> {
    pub name: &'a str,
    pub brief: &'a Path,
    pub budget: f64,
    pub model: &'a str,
}

/// Runs `order`; fails unless the session ends with status 0.
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
    crate::api_fds::refuse_inherited()?;
    let config = crate::config::Config::load().map_err(|e| e.to_string())?;
    let budget = Budget::new(order.budget, crate::api_prices::prices(&config)?);
    budget.admit(order.model)?;
    let builder = Builder::new(order.name, &crate::api_host::home()?)?;
    let secret = crate::api_build_setup::key(&builder.home.join(KEY_FILE))?;
    let (main, objects) = crate::api_clone::main_repository()?;
    let branch = crate::api_fences_run::branch(&builder);
    crate::api_clone::make(&builder, (&main, &objects), &branch)?;
    let staging = crate::api_build_setup::private(&builder.root().join("requests"))?;
    let routes = Arc::new(Routes {
        proxy: Some(Proxy::new(secret, budget, staging)),
        ..Routes::default()
    });
    crate::api_server::serve(&builder.socket(), Arc::clone(&routes))?;
    let host = crate::api_host::host(objects)?;
    let mut lines = proven(&builder, &host)?;
    let (said, ended) = session(order, (&builder, &host), &routes)?;
    lines.extend(said);
    Ok((lines, ended))
}

/// The session run, and the lines that say how it ended.
fn session(
    order: &Order<'_>,
    (builder, host): (&Builder, &Host),
    routes: &Routes,
) -> Result<(Vec<String>, Ended), String> {
    let claude = Path::new(BIN).join("claude").display().to_string();
    let mut command = vec![claude];
    command.extend(crate::api_session::arguments(order.model, order.budget));
    let words: Vec<&str> = command.iter().map(String::as_str).collect();
    let result = builder.root().join("result.json");
    let proxy = routes.proxy.as_ref().ok_or("the run has no proxy")?;
    let session = sandboxed(builder, host, &words);
    let ended = crate::api_session::run(session, (order.brief, &result), &proxy.budget)?;
    let lines = crate::api_build_report::summary(&ended, (&result, builder), routes, proxy)?;
    Ok((lines, ended))
}
