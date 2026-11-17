//! The file descriptors this xtask would hand the sandbox (review of #467,
//! M1). bwrap passes every inherited descriptor through, and Rust opens
//! its own with close-on-exec, so the only ones a child inherits are
//! those this process was given without it (a pipe or file held by the
//! spawning harness or a hook). The sandbox is not started while any is
//! open; its stdin is /dev/null (`api_fences_run.rs` `sandboxed`).

use std::path::Path;

/// `O_CLOEXEC` as `/proc/<pid>/fdinfo` shows it in `flags`, octal.
const CLOEXEC: u32 = 0o2_000_000;

/// The descriptors above 2 without close-on-exec in `fdinfo` (a
/// `/proc/<pid>/fdinfo` directory), each with what it is in `fd`.
///
/// # Errors
///
/// A directory that cannot be read, or an fdinfo entry of a shape this
/// does not read, naming it.
pub fn inherited(fdinfo: &Path, fd: &Path) -> Result<Vec<String>, String> {
    let entries =
        std::fs::read_dir(fdinfo).map_err(|e| format!("reading {}: {e}", fdinfo.display()))?;
    let mut open = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("reading {}: {e}", fdinfo.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(number) = name.parse::<u32>() else {
            continue;
        };
        if number > 2 && !closed_on_exec(&entry.path())? {
            let target = std::fs::read_link(fd.join(&name))
                .map_or_else(|e| e.to_string(), |t| t.display().to_string());
            open.push(format!("fd {number} ({target})"));
        }
    }
    open.sort();
    Ok(open)
}

/// Whether the fdinfo file at `path` says close-on-exec. A descriptor
/// closed between the listing and this read (its file gone) is not
/// inherited either; any other failure to read it is an error.
fn closed_on_exec(path: &Path) -> Result<bool, String> {
    let info = match std::fs::read_to_string(path) {
        Ok(info) => info,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(e) => return Err(format!("reading {}: {e}", path.display())),
    };
    let flags = info
        .lines()
        .find_map(|line| line.strip_prefix("flags:"))
        .and_then(|flags| u32::from_str_radix(flags.trim(), 8).ok())
        .ok_or_else(|| format!("{} has no octal `flags:` line: {info:?}", path.display()))?;
    Ok(flags & CLOEXEC != 0)
}

/// Refuses while this process holds a descriptor a child would inherit.
///
/// # Errors
///
/// Each such descriptor, named, with the fix.
pub fn refuse_inherited() -> Result<(), String> {
    let open = inherited(Path::new("/proc/self/fdinfo"), Path::new("/proc/self/fd"))?;
    if open.is_empty() {
        return Ok(());
    }
    Err(format!(
        "this xtask holds {} without close-on-exec, which the sandbox would inherit; fix: run it \
         without them (close them in the caller, or start it from a plain shell)",
        open.join(", ")
    ))
}

#[cfg(test)]
#[path = "api_fds_tests.rs"]
mod tests;
