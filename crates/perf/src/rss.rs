//! This process's resident set, for the Mount workload's memory rows.
//! Linux only: it is read from /proc/self/status; elsewhere it is
//! refused, naming the platform, and never reported as zero.

const STATUS: &str = "/proc/self/status";

/// The resident set in KiB.
///
/// # Errors
/// Not Linux (the message names the platform), /proc/self/status
/// unreadable, or without a `VmRSS` line in kB.
pub fn resident_kib() -> Result<u64, String> {
    status_kib("VmRSS")
}

/// The anonymous part of the resident set in KiB (`RssAnon`): heap and
/// stacks, without file pages (code, fonts, shader caches).
///
/// # Errors
/// As [`resident_kib`], for the `RssAnon` line.
pub fn anon_kib() -> Result<u64, String> {
    status_kib("RssAnon")
}

/// The most the resident set has been, in KiB (`VmHWM`).
///
/// # Errors
/// As [`resident_kib`], for the `VmHWM` line.
pub fn peak_kib() -> Result<u64, String> {
    status_kib("VmHWM")
}

fn status_kib(field: &str) -> Result<u64, String> {
    if !cfg!(target_os = "linux") {
        return Err(format!(
            "memory is read from {STATUS}, which {} has not; run the Mount workload on Linux",
            std::env::consts::OS
        ));
    }
    let status = std::fs::read_to_string(STATUS).map_err(|e| format!("reading {STATUS}: {e}"))?;
    parse_status_kib(&status, field)
}

/// The `field` line of `status` in KiB.
///
/// # Errors
/// No such line, or one that is not a count of kB (the message names it).
pub fn parse_status_kib(status: &str, field: &str) -> Result<u64, String> {
    let prefix = format!("{field}:");
    let line = status
        .lines()
        .find(|line| line.starts_with(&prefix))
        .ok_or_else(|| format!("{STATUS} has no {field} line"))?;
    line.strip_prefix(&prefix)
        .and_then(|rest| rest.trim().strip_suffix("kB"))
        .and_then(|count| count.trim().parse().ok())
        .ok_or_else(|| format!("{STATUS}: cannot read {line:?} as a count of kB"))
}

#[cfg(test)]
#[path = "rss_tests.rs"]
mod tests;
