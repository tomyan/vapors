//! A server already running, started under a run's TMPDIR that is now
//! gone, fails every compile with exit status 254 (vapors #117). Such a
//! server cannot come from this xtask, which starts the server with
//! TMPDIR unset (`sccache_start.rs`), but one started by an earlier
//! xtask, or by a build outside xtask, can still be running. This finds
//! it: each `sccache` process of this user is read from
//! `/proc/<pid>/environ`; one that is a server (`SCCACHE_START_SERVER=1`)
//! on this xtask's port with a TMPDIR under `xtask-run-` that no longer
//! exists is refused, naming the fix. A process that cannot be read is
//! left alone.

use std::path::Path;

/// sccache's default server port.
const DEFAULT_PORT: &str = "4226";

/// Refuse a running server whose run TMPDIR is gone.
///
/// # Errors
/// One is running, naming it and the fix.
pub fn check() -> Result<(), String> {
    let port = std::env::var("SCCACHE_SERVER_PORT").unwrap_or_else(|_| DEFAULT_PORT.to_owned());
    let Ok(listed) = std::fs::read_dir("/proc") else {
        return Ok(());
    };
    for entry in listed.flatten() {
        let dir = entry.path();
        let comm = std::fs::read_to_string(dir.join("comm")).unwrap_or_default();
        if comm.trim() != "sccache" {
            continue;
        }
        let Ok(environ) = std::fs::read(dir.join("environ")) else {
            continue;
        };
        if let Some(why) = stale(&environ, &port, &|path| path.exists()) {
            return Err(why);
        }
    }
    Ok(())
}

/// Why the process with `environ` is a stale server on `port`, if it is.
#[must_use]
pub fn stale(environ: &[u8], port: &str, exists: &dyn Fn(&Path) -> bool) -> Option<String> {
    let vars: Vec<String> = environ
        .split(|&b| b == 0)
        .map(|v| String::from_utf8_lossy(v).into_owned())
        .collect();
    let get = |name: &str| {
        vars.iter()
            .find_map(|v| v.strip_prefix(&format!("{name}=")))
    };
    let server = get("SCCACHE_START_SERVER") == Some("1");
    let its_port = get("SCCACHE_SERVER_PORT").unwrap_or(DEFAULT_PORT);
    let tmpdir = get("TMPDIR")?;
    let from_a_run = Path::new(tmpdir)
        .file_name()
        .is_some_and(|n| n.to_string_lossy().starts_with("xtask-run-"));
    if !server || its_port != port || !from_a_run || exists(Path::new(tmpdir)) {
        return None;
    }
    Some(format!(
        "the sccache server on port {port} runs in TMPDIR {tmpdir}, which is gone, so every \
         compile through it fails; fix: sccache --stop-server (xtask starts a new one)"
    ))
}

#[cfg(test)]
#[path = "sccache_stale_tests.rs"]
mod tests;
