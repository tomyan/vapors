//! The property table: every longhand with its initial value, inherited
//! flag and value grammar; the shorthands that expand into them; and the
//! parse of a declaration into typed longhand values.

mod declaration;
mod names;
mod shorthand;
mod shorthand_border;
mod shorthand_layout;
mod shorthand_motion;
mod table;

use std::sync::OnceLock;

use crate::syntax::{Grammar, parse_grammar};
use crate::{Error, Position, Result};

pub use declaration::{Longhand, Specified, parse_declaration, resolve_pending};
pub use names::IGNORED_VENDOR_PROPERTIES;
pub use shorthand::{SHORTHANDS, is_shorthand};
pub use table::{PROPERTIES, PropertyDef};

/// A longhand property: an index into [`PROPERTIES`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PropertyId(u16);

impl PropertyId {
    /// The id of a longhand by name; `None` for shorthands and unknowns.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        PROPERTIES
            .binary_search_by_key(&name, |d| d.name)
            .ok()
            .map(Self::at)
    }

    /// Every longhand, in table order.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..PROPERTIES.len()).map(Self::at)
    }

    #[allow(clippy::cast_possible_truncation)] // the table has about a hundred entries
    fn at(index: usize) -> Self {
        Self(index as u16)
    }

    /// The index into [`PROPERTIES`].
    #[must_use]
    pub fn index(self) -> usize {
        usize::from(self.0)
    }

    /// The table entry.
    #[must_use]
    pub fn def(self) -> &'static PropertyDef {
        &PROPERTIES[self.index()]
    }

    /// The property name.
    #[must_use]
    pub fn name(self) -> &'static str {
        self.def().name
    }

    /// Whether the property inherits by default.
    #[must_use]
    pub fn inherited(self) -> bool {
        self.def().inherited
    }

    /// The parsed value grammar, built once.
    #[must_use]
    pub fn grammar(self) -> &'static Grammar {
        static GRAMMARS: OnceLock<Vec<Grammar>> = OnceLock::new();
        &GRAMMARS.get_or_init(|| PROPERTIES.iter().map(|d| parse_grammar(d.syntax)).collect())
            [self.index()]
    }
}

/// A property name as the table spells it: lowercased, unless custom.
#[must_use]
pub fn canonical_name(name: &str) -> String {
    if name.starts_with("--") {
        return name.to_owned();
    }
    name.to_ascii_lowercase()
}

/// Whether `name` is a longhand, a shorthand or a custom property.
#[must_use]
pub fn is_supported(name: &str) -> bool {
    name.starts_with("--") || PropertyId::from_name(name).is_some() || is_shorthand(name)
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
    if let Some(hint) = names::unsupported_shorthand_hint(name) {
        return Err(Error::new(
            position,
            format!("shorthand {name:?} is not supported; {hint}"),
        ));
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
