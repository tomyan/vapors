//! Where an API builder's files are (`docs/plans/api-builder.md`): its
//! own directory under `~/.cache/api-build/<name>`, holding its clone,
//! its cargo home and the harness socket, and its target directory with
//! the others under `~/.cache/vapors-targets`.

use std::path::{Path, PathBuf};

/// The API key's file, relative to the home directory: mode 600, read
/// only outside the sandbox (the fences prove it unreadable inside).
pub const KEY_FILE: &str = ".csvg-tok";

/// One builder's paths, from its name and the home directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Builder {
    pub name: String,
    pub home: PathBuf,
}

impl Builder {
    /// Builder `name` under `home`.
    ///
    /// # Errors
    ///
    /// A name that is empty or holds anything but ASCII letters, digits
    /// and `-`: it names directories and a branch.
    pub fn new(name: &str, home: &Path) -> Result<Self, String> {
        let plain = |c: char| c.is_ascii_alphanumeric() || c == '-';
        if name.is_empty() || !name.chars().all(plain) {
            return Err(format!(
                "builder name {name:?}: use ASCII letters, digits and `-` only, as it names \
                 directories and a branch"
            ));
        }
        Ok(Self {
            name: name.to_owned(),
            home: home.to_owned(),
        })
    }

    /// Everything of this builder's but its target directory.
    pub fn root(&self) -> PathBuf {
        self.home.join(".cache/api-build").join(&self.name)
    }

    /// The builder's clone, the one directory of the repository it
    /// writes.
    pub fn clone_dir(&self) -> PathBuf {
        self.root().join("vapors")
    }

    /// Its target directory, beside the other worktrees'.
    pub fn target(&self) -> PathBuf {
        self.home
            .join(".cache/vapors-targets")
            .join(format!("api-{}", self.name))
    }

    /// The sccache directory the sandboxes share, which the host never
    /// reads (`api_sandbox.rs`).
    pub fn sandbox_sccache(&self) -> PathBuf {
        self.home.join(".cache/sccache-sandbox")
    }

    /// The cargo home it writes, the registry bound read-only inside.
    pub fn cargo_home(&self) -> PathBuf {
        self.root().join("cargo-home")
    }

    /// The directory of the harness socket, bound read-only inside.
    pub fn socket_dir(&self) -> PathBuf {
        self.root().join("socket")
    }

    /// The harness socket: the session's one way out.
    pub fn socket(&self) -> PathBuf {
        self.socket_dir().join("harness.sock")
    }
}

#[cfg(test)]
#[path = "api_paths_tests.rs"]
mod tests;
