//! Every job ends with the step that reports a timeout: GitHub shows a
//! job that hit `timeout-minutes` as cancelled, which says nothing. The
//! report step names the timeout and the step it stopped.

use std::time::Duration;

use serde_json::Value;

/// The report's name; `.github/scripts/report-timeout.sh` leaves the step
/// of this name out when it looks for the last step to start.
const NAME: &str = "report a timeout";

/// Refuses job `job` unless its last step is [`NAME`], `if: cancelled()`, with env
/// `TIMEOUT_MINUTES` equal to the job's `timeout-minutes`.
pub fn check_timeout_report(job: &str, timeout: Duration, steps: &[Value]) -> Result<(), String> {
    let minutes = timeout.as_secs() / 60;
    let last = steps.last();
    let text = |key: &str| last.and_then(|step| step.get(key)).and_then(Value::as_str);
    let is_report = text("if") == Some("cancelled()") && text("name") == Some(NAME);
    if !is_report {
        return Err(format!(
            "{job}: the last step is not the timeout report; fix: end the job with a step \
             `name: {NAME}`, `if: cancelled()`, env `TIMEOUT_MINUTES: {minutes}`, \
             run `.github/scripts/report-timeout.sh` (see the other jobs)"
        ));
    }
    check_minutes(
        job,
        minutes,
        last.and_then(|step| step.pointer("/env/TIMEOUT_MINUTES")),
    )
}

fn check_minutes(job: &str, minutes: u64, reported: Option<&Value>) -> Result<(), String> {
    let reported = reported.map(|value| match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    });
    if reported.as_deref() == Some(minutes.to_string().as_str()) {
        return Ok(());
    }
    Err(format!(
        "{job}: the timeout report's TIMEOUT_MINUTES is {}, but the job's timeout-minutes is \
         {minutes}; fix: make them equal",
        reported.as_deref().unwrap_or("unset")
    ))
}

#[cfg(test)]
#[path = "gate_report_tests.rs"]
mod tests;
