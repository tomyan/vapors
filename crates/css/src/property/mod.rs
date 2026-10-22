//! The property table: every supported property by name. Later commits
//! add each property's initial value, inherited flag and value syntax.

mod names;

use crate::{Error, Position, Result};

pub use names::{IGNORED_VENDOR_PROPERTIES, SUPPORTED_PROPERTIES};

/// A property name as the table spells it: lowercased, unless custom.
#[must_use]
pub fn canonical_name(name: &str) -> String {
    if name.starts_with("--") {
        return name.to_owned();
    }
    name.to_ascii_lowercase()
}

/// Whether `name` is a supported property or a custom property.
#[must_use]
pub fn is_supported(name: &str) -> bool {
    name.starts_with("--") || SUPPORTED_PROPERTIES.contains(&name)
}

/// Whether `name` is a vendor-prefixed property the crate drops on purpose.
#[must_use]
pub fn is_ignored(name: &str) -> bool {
    IGNORED_VENDOR_PROPERTIES.iter().any(|(n, _)| *n == name)
}

/// Refuse a property name the table does not list, naming the fix.
///
/// # Errors
/// Unknown properties and sumi's terminal extras.
pub fn check_name(name: &str, position: Position) -> Result<()> {
    if is_supported(name) {
        return Ok(());
    }
    if let Some(hint) = names::sumi_extra_hint(name) {
        return Err(Error::new(
            position,
            format!(
                "property {name:?} is a sumi terminal extra and does not exist in vapors; {hint}"
            ),
        ));
    }
    Err(Error::new(
        position,
        format!("unknown property {name:?}; see docs/css.md for the supported list"),
    ))
}
