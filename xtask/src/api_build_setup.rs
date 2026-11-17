//! What `api-build` sets up outside the sandbox before a session: the key,
//! read from a file that must be private, and private directories.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::ux_secret::Secret;

/// The key in `file`, which must be mode 600.
///
/// # Errors
///
/// The file missing, readable by others, or not a key.
pub fn key(file: &Path) -> Result<Secret, String> {
    let metadata =
        fs::metadata(file).map_err(|e| format!("the key file {}: {e}", file.display()))?;
    let mode = metadata.permissions().mode() & 0o777;
    if mode != 0o600 {
        return Err(format!(
            "the key file {} is mode {mode:o}; fix: chmod 600 {}",
            file.display(),
            file.display()
        ));
    }
    let key = fs::read_to_string(file)
        .map_err(|e| format!("reading the key file {}: {e}", file.display()))?;
    Secret::new(key.trim().to_owned())
}

/// `dir` made, private to this user.
///
/// # Errors
///
/// The filesystem refusing, naming `dir`.
pub fn private(dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| format!("making {}: {e}", dir.display()))?;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
        .map_err(|e| format!("making {} private: {e}", dir.display()))?;
    Ok(dir.to_owned())
}

#[cfg(test)]
#[path = "api_build_setup_tests.rs"]
mod tests;
