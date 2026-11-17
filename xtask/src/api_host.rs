//! What an API builder's sandbox reads from the host, found once a run
//! (`api_sandbox.rs` `Host`): this xtask binary, the CLI, the main
//! repository's objects and the GPU's render nodes.

use std::path::{Path, PathBuf};

use crate::api_sandbox::Host;

/// The paths outside the sandbox reads, the main repository's objects
/// being `objects`.
///
/// # Errors
///
/// This binary, the CLI or `/dev/dri` not found or not read.
pub fn host(objects: PathBuf) -> Result<Host, String> {
    let xtask = std::env::current_exe().map_err(|e| format!("finding this xtask binary: {e}"))?;
    Ok(Host {
        xtask,
        claude: claude()?,
        objects,
        render_nodes: render_nodes(Path::new("/dev/dri"))?,
    })
}

/// The render nodes in `dri`; none when it does not exist, as on a
/// machine with no GPU.
///
/// # Errors
///
/// `dri` there but not read.
pub fn render_nodes(dri: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = match std::fs::read_dir(dri) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("reading {}: {e}", dri.display())),
    };
    let render = |path: &PathBuf| {
        path.file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("renderD"))
    };
    let mut nodes: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(render)
        .collect();
    nodes.sort();
    Ok(nodes)
}

/// The CLI's binary, its symlinks resolved.
fn claude() -> Result<PathBuf, String> {
    let link = home()?.join(".local/bin/claude");
    std::fs::canonicalize(&link).map_err(|e| {
        format!(
            "the claude CLI is not at {} ({e}); fix: install it (claude.com/claude-code)",
            link.display()
        )
    })
}

/// The home directory.
///
/// # Errors
///
/// `HOME` unset.
pub fn home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set: the builder's directories are under it".to_owned())
}

#[cfg(test)]
#[path = "api_host_tests.rs"]
mod tests;
