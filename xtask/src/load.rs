//! The machine's load, recorded around each gate step, so a timing
//! failure shows the load it ran under.

/// The one-minute load average from `/proc/loadavg`.
pub fn now() -> Result<String, String> {
    let text =
        std::fs::read_to_string("/proc/loadavg").map_err(|e| format!("/proc/loadavg: {e}"))?;
    parse(&text)
}

/// The first field of `/proc/loadavg`'s text, a decimal number.
pub fn parse(text: &str) -> Result<String, String> {
    let first = text.split_whitespace().next().unwrap_or_default();
    if first.parse::<f64>().is_ok() {
        return Ok(first.to_owned());
    }
    Err(format!(
        "/proc/loadavg reads `{}`, not a load average",
        text.trim()
    ))
}

/// The line the gate prints after a step.
pub fn line(start: &Result<String, String>, end: &Result<String, String>) -> String {
    let shown = |load: &Result<String, String>| match load {
        Ok(load) => load.clone(),
        Err(why) => format!("unreadable: {why}"),
    };
    format!(
        "     load {} at the start, {} at the end",
        shown(start),
        shown(end)
    )
}

#[cfg(test)]
#[path = "load_tests.rs"]
mod tests;
