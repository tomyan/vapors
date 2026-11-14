//! The state of Mesa's on-disk shader cache when the 60 s run starts.
//! Frame 0's pipeline creation compiles shaders, or finds them in that
//! cache, so the run states which, and refuses to run with Mesa's
//! default cache, whose state nobody chose.

use std::path::{Path, PathBuf};

/// The cache as the run found it.
#[derive(Debug, PartialEq, Eq)]
pub enum ShaderCache {
    /// `MESA_SHADER_CACHE_DISABLE=true`: every pipeline compiled.
    Disabled,
    /// `MESA_SHADER_CACHE_DIR`, with how many entries it held.
    Dir {
        /// The directory.
        path: PathBuf,
        /// Its entries at the start: 0 is a cold cache.
        entries: usize,
    },
}

impl ShaderCache {
    /// One line for the run's header.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Disabled => "disabled (MESA_SHADER_CACHE_DISABLE=true)".into(),
            Self::Dir { path, entries: 0 } => {
                format!("{}, empty at the start (cold)", path.display())
            }
            Self::Dir { path, entries } => {
                format!("{}, {entries} entries at the start (warm)", path.display())
            }
        }
    }
}

/// The cache that the environment `var` names.
///
/// # Errors
/// Neither variable is set, or the directory cannot be read.
pub fn shader_cache(var: impl Fn(&str) -> Option<String>) -> Result<ShaderCache, String> {
    if var("MESA_SHADER_CACHE_DISABLE").as_deref() == Some("true") {
        return Ok(ShaderCache::Disabled);
    }
    let Some(dir) = var("MESA_SHADER_CACHE_DIR") else {
        return Err(String::from(
            "Mesa's shader cache is its default one, whose state frame 0 depends on; set \
             MESA_SHADER_CACHE_DIR to a new empty directory for a cold run (or one a run has \
             filled, for a warm run), or MESA_SHADER_CACHE_DISABLE=true, such as \
             MESA_SHADER_CACHE_DIR=$(mktemp -d) before the command",
        ));
    };
    let path = PathBuf::from(dir);
    let entries = entries(&path)?;
    Ok(ShaderCache::Dir { path, entries })
}

fn entries(path: &Path) -> Result<usize, String> {
    let read = std::fs::read_dir(path).map_err(|error| {
        format!(
            "MESA_SHADER_CACHE_DIR {} cannot be read: {error}; create it",
            path.display()
        )
    })?;
    Ok(read.count())
}

#[cfg(test)]
#[path = "shader_cache_tests.rs"]
mod tests;
