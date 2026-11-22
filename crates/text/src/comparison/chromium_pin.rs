//! The one Chromium every comparison uses, local and in CI alike: the
//! build pinned under `[downloads] chromium` in `.config/tools.toml`,
//! installed by `cargo xtask setup` after its archive's and binary's
//! SHA-256 match, at `~/.local/share/factory/chromium-<version>/<member>`
//! (xtask's `downloads::tree_root`). The version and member are read from
//! that file at compile time, so there is one pin. `VAPORS_CHROMIUM`
//! overrides it, for a diagnosis by hand.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The tools file, as built: the pin's one source.
const TOOLS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../.config/tools.toml"
));

/// Where the pinned build's binary is installed under `home`.
///
/// # Panics
/// When the tools file pins no `chromium.version` or `chromium.member`:
/// a build of this crate from a tree without the pin is refused at its
/// first use, naming the fix.
#[must_use]
pub fn pinned_binary(home: &Path) -> PathBuf {
    let version = pinned_field(TOOLS, "version").expect(UNPINNED);
    let member = pinned_field(TOOLS, "member").expect(UNPINNED);
    home.join(".local/share/factory")
        .join(format!("chromium-{version}"))
        .join(member)
}

const UNPINNED: &str =
    ".config/tools.toml pins no [downloads] chromium.version and chromium.member; fix: add them";

/// The quoted value of `chromium.<field>` under `[downloads]` in `tools`.
pub(super) fn pinned_field<'a>(tools: &'a str, field: &str) -> Option<&'a str> {
    let key = format!("chromium.{field}");
    let mut inside = false;
    tools.lines().map(str::trim).find_map(|line| {
        if line.starts_with('[') {
            inside = line == "[downloads]";
            return None;
        }
        let (name, value) = line.split_once('=')?;
        (inside && name.trim() == key).then(|| value.trim().trim_matches('"'))
    })
}

/// The binary to launch: `override_` when given, else the pinned build
/// under `home` when `installed` says it is there.
///
/// # Errors
/// The pinned path, when there is no override and it is not installed.
pub(super) fn resolve(
    override_: Option<OsString>,
    home: Option<&Path>,
    installed: impl Fn(&Path) -> bool,
) -> Result<PathBuf, PathBuf> {
    if let Some(binary) = override_ {
        return Ok(PathBuf::from(binary));
    }
    let pinned = pinned_binary(home.unwrap_or(Path::new("$HOME")));
    if home.is_some() && installed(&pinned) {
        Ok(pinned)
    } else {
        Err(pinned)
    }
}

#[cfg(test)]
#[path = "chromium_pin_tests.rs"]
mod tests;
