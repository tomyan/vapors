//! The one expression the gate models in `env:`: `${{ github.token }}`.
//! CI gives the job's read-only token; the gate gives the local
//! `GITHUB_TOKEN`, or leaves the variable unset and says so. Every other
//! expression is refused.

/// The only expression the gate evaluates, written exactly so.
pub const GITHUB_TOKEN: &str = "${{ github.token }}";

/// The value of env `name` in step `label` when the gate runs it: the
/// text itself, the local token for [`GITHUB_TOKEN`] (`None`, unset,
/// when there is none), or a refusal.
pub fn env_text(
    text: &str,
    name: &str,
    label: &str,
    local_token: Option<&str>,
) -> Result<Option<String>, String> {
    if text == GITHUB_TOKEN {
        return Ok(local_token.map(str::to_owned));
    }
    if text.contains("${{") {
        return Err(format!(
            "{label}: env {name} is an expression, which the gate cannot evaluate; \
             the only one it models is `{GITHUB_TOKEN}`, written exactly so"
        ));
    }
    Ok(Some(text.to_owned()))
}

/// Printed when [`GITHUB_TOKEN`] is left unset.
pub fn unauthenticated_note(name: &str, label: &str) -> String {
    format!(
        "note {label}: GITHUB_TOKEN is not set here, so env {name} is left unset \
         and the step runs unauthenticated"
    )
}

#[cfg(test)]
#[path = "gate_expression_tests.rs"]
mod tests;
