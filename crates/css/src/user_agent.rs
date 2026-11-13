//! The user-agent sheet: the defaults every document starts from. It
//! lives here, as text, so the runtime's cascade and the compiler's
//! build-time analysis read one sheet.

use crate::ast::Stylesheet;

/// The sheet's text (`ua.css` beside this file).
pub const TEXT: &str = include_str!("ua.css");

/// The sheet, parsed.
///
/// # Panics
/// When `ua.css` does not parse, naming the position: a defect in this
/// crate, which `cargo test -p vapors-css` reproduces.
#[must_use]
pub fn sheet() -> Stylesheet {
    crate::parse(TEXT).unwrap_or_else(|error| {
        panic!(
            "vapors-css's ua.css does not parse at {}: {}",
            error.position, error.message
        )
    })
}
