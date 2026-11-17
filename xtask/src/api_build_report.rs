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

/// The summary's lines.
///
/// # Errors
///
/// The proxy's count and the CLI's report disagreeing.
pub fn summary(
    ended: &Ended,
    (result, builder): (&Path, &Builder),
    routes: &Routes,
    proxy: &Proxy,
) -> Result<Vec<String>, String> {
    let (dollars, unknown) = proxy.budget.spent();
    let (kept, count) = routes.refused.kept();
    let refused = format!("{count}: {}", kept.join("; "));
    let unknown = unknown
        .map(|why| format!(" (then unknown: {why})"))
        .unwrap_or_default();
    let hellos = routes.hellos.load(Ordering::Relaxed);
    let lines = vec![
        format!("session: {ended:?}"),
        format!("spent: ${dollars:.4} counted by the proxy{unknown}"),
        format!("refused: {refused}"),
        format!("HEAD /api/hello answered here: {hellos}"),
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

/// The proxy's count and the CLI's report, within a hundredth of a cent,
/// as both price the same tokens; a report the CLI did not write is not
/// compared (a session stopped by the harness writes none).
///
/// # Errors
///
/// The two differing.
pub fn agreed(proxy: f64, cli: Option<f64>) -> Result<(), String> {
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
    let text = std::fs::read_to_string(result).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let entries = value.as_array().cloned().unwrap_or_else(|| vec![value]);
    entries.iter().find(|entry| entry["type"] == "result")?["total_cost_usd"].as_f64()
}

#[cfg(test)]
#[path = "api_build_report_tests.rs"]
mod tests;
