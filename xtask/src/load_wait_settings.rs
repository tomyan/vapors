//! `[load]` in `.config/factory.toml`: whether `ci` and `gate` wait for
//! the machine's load, over what, and for how long.

use std::time::Duration;

use crate::config::Config;

/// The deadline when `[load] deadline_minutes` is unset: the 10 minutes
/// agents waited by hand before this was mechanical; Tom chose to go on
/// at it, warning (docs/decisions/2026-11-01-load-wait-deadline.md).
/// Not a measurement.
pub const DEFAULT_DEADLINE: Duration = Duration::from_mins(10);

/// When to wait and for how long.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    /// `[load] wait`: false turns the wait off.
    pub enabled: bool,
    /// Wait while the runnable tasks (this reader included) are over this.
    pub threshold: f64,
    /// Then go on, saying so.
    pub deadline: Duration,
}

/// `[load] wait`, `threshold` and `deadline_minutes`; the wait is on by
/// default, the threshold defaults to `cores`, the deadline to
/// [`DEFAULT_DEADLINE`].
///
/// # Errors
///
/// A value of the wrong shape, or a deadline too large to wait for,
/// naming the key and the fix.
pub fn settings(config: &Config, cores: usize) -> Result<Settings, String> {
    let enabled = match config.get("load.wait") {
        None | Some("true") => true,
        Some("false") => false,
        Some(other) => return Err(refusal("wait", other, "true or false")),
    };
    let threshold = match config.get("load.threshold") {
        None => f64::from(u32::try_from(cores).unwrap_or(u32::MAX)),
        Some(text) => positive("threshold", text)?,
    };
    let deadline = match config.get("load.deadline_minutes") {
        None => DEFAULT_DEADLINE,
        Some(text) => minutes(text)?,
    };
    Ok(Settings {
        enabled,
        threshold,
        deadline,
    })
}

fn minutes(text: &str) -> Result<Duration, String> {
    let value = positive("deadline_minutes", text)?;
    Duration::try_from_secs_f64(value * 60.0).map_err(|_| {
        refusal(
            "deadline_minutes",
            text,
            "a number of minutes small enough to wait",
        )
    })
}

fn positive(key: &str, text: &str) -> Result<f64, String> {
    match text.parse::<f64>() {
        Ok(value) if value.is_finite() && value > 0.0 => Ok(value),
        _ => Err(refusal(key, text, "a positive number")),
    }
}

fn refusal(key: &str, text: &str, wanted: &str) -> String {
    format!("[load] {key} is `{text}`, not {wanted}; fix: .config/factory.toml")
}

#[cfg(test)]
#[path = "load_wait_settings_tests.rs"]
mod tests;
