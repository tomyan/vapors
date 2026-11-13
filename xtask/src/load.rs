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
    if first.parse::<f64>().is_ok_and(f64::is_finite) {
        return Ok(first.to_owned());
    }
    Err(format!(
        "/proc/loadavg reads `{}`, not a load average",
        text.trim()
    ))
}

/// The count of runnable tasks now, this reader included, from
/// `/proc/loadavg`: what the load wait reads.
pub fn runnable_now() -> Result<u32, String> {
    runnable_in(std::path::Path::new("/proc/loadavg"))
}

/// The runnable count in the file at `path`, read as `/proc/loadavg`.
pub fn runnable_in(path: &std::path::Path) -> Result<u32, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    runnable(&text)
}

/// The runnable count in `/proc/loadavg`'s fourth field, `runnable/total`.
pub fn runnable(text: &str) -> Result<u32, String> {
    let fourth = text.split_whitespace().nth(3).unwrap_or_default();
    let count = fourth
        .split_once('/')
        .map(|(count, _)| count.parse::<u32>());
    if let Some(Ok(count)) = count {
        return Ok(count);
    }
    Err(format!(
        "/proc/loadavg reads `{}`, whose fourth field is not runnable/total",
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
