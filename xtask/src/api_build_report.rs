//! What `api-build` prints once the session ends: how it ended, what the
//! proxy counted and refused, how often the CLI's connectivity check was
//! answered here, and where the result and clone are. The proxy's count
//! and the CLI's reported cost are two answers to one question, held
//! together.

use std::path::Path;
use std::sync::atomic::Ordering;

use crate::api_paths::Builder;
use crate::api_server::Routes;
use crate::api_server_proxy::Proxy;
use crate::api_session::Ended;

/// The summary's lines, and whether the proxy's count and the CLI's
/// report agree; the lines are kept either way, as they are what
/// explains a disagreement (review of #476-#478, M6).
pub fn summary(
    ended: &Ended,
    (result, builder): (&Path, &Builder),
    routes: &Routes,
    proxy: &Proxy,
) -> (Vec<String>, Result<(), String>) {
    let (dollars, in_flight) = proxy.budget.spent();
    let (kept, count) = routes.refused.kept();
    let stopped = proxy.budget.stopped().unwrap_or_default();
    let lines = vec![
        format!("session: {ended:?}"),
        format!(
            "spent: ${dollars:.4} counted by the proxy, ${in_flight:.4} still in flight; {stopped}"
        ),
        format!("refused: {count}: {}", kept.join("; ")),
        format!(
            "HEAD /api/hello answered here: {}",
            routes.hellos.load(Ordering::Relaxed)
        ),
        format!("result: {}", result.display()),
        format!(
            "clone: {} (branch api/{})",
            builder.clone_dir().display(),
            builder.name
        ),
    ];
    let whole = *ended == Ended::Exited(Some(0));
    (lines, agreed(dollars, reported_cost(result), whole))
}

/// The proxy's count and the CLI's report, within a hundredth of a cent,
/// as both price the same tokens. A session that ended with status 0
/// must have reported its cost: no cost is a failure, not a pass. One
/// that did not end so (stopped by the harness) may have written none.
/// The CLI's report is the session's to write, so this checks the
/// prices, not the session.
///
/// # Errors
///
/// The two differing, or no report from a session that ended with 0.
pub fn agreed(proxy: f64, cli: Option<f64>, whole: bool) -> Result<(), String> {
    match cli {
        Some(cli) if (proxy - cli).abs() > 0.0001 => Err(format!(
            "the proxy counted ${proxy:.4} and the CLI reported ${cli:.4}: one of them prices \
             the tokens wrongly; compare [api_build.prices] with the CLI's modelUsage in the result"
        )),
        None if whole => Err(
            "the session ended with status 0 but its result holds no total_cost_usd, so the \
             proxy's count has nothing to agree with; read the result, and the CLI's version"
                .into(),
        ),
        _ => Ok(()),
    }
}

/// The cost the CLI's JSON result reports, if it holds one.
fn reported_cost(result: &Path) -> Option<f64> {
    let text = std::fs::read_to_string(result).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let entries = value.as_array().cloned().unwrap_or_else(|| vec![value]);
    entries.iter().find(|entry| entry["type"] == "result")?["total_cost_usd"].as_f64()
}

#[cfg(test)]
#[path = "api_build_report_tests.rs"]
mod tests;
