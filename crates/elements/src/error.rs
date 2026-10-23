//! What can go wrong between the generated code's calls and a painted
//! scene. Each error names what to change.

use std::fmt;

/// An error found while building, styling or laying out the tree.
#[derive(Clone, Debug, PartialEq)]
pub enum ElementError {
    /// A tag the tree does not implement yet.
    UnsupportedTag(String),
    /// A listener for an event the tree never fires.
    UnfiredEvent(String),
    /// A stylesheet that does not parse, or a declaration that does not
    /// compute (an unresolved `var()`).
    Css {
        /// The sheet's name, as given to `add_stylesheet`, or the cascade.
        sheet: String,
        /// The CSS crate's refusal, with its line and column.
        error: vapors_css::Error,
    },
    /// A property, or a value of one, the tree does not implement yet.
    Unhonoured {
        /// The element, as `<tag class="..">`.
        element: String,
        /// The property name.
        property: &'static str,
        /// The computed value, as CSS text.
        value: String,
        /// Why, and the slice that brings it.
        reason: String,
    },
}

impl fmt::Display for ElementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedTag(message) | Self::UnfiredEvent(message) => f.write_str(message),
            Self::Css { sheet, error } => write!(
                f,
                "{sheet}:{}:{}: {}",
                error.position.line, error.position.column, error.message
            ),
            Self::Unhonoured {
                element,
                property,
                value,
                reason,
            } => write!(f, "{element}: `{property}: {value}` {reason}"),
        }
    }
}

impl std::error::Error for ElementError {}
