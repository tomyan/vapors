//! The command letters of the subset.

/// A command letter of the subset.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verb {
    Move,
    Line,
    Horizontal,
    Vertical,
    Cubic,
    Quadratic,
    Arc,
    Close,
}

impl Verb {
    /// The verb `letter` names and whether it is relative.
    pub(crate) fn of(letter: u8) -> Option<(Self, bool)> {
        let verb = match letter.to_ascii_uppercase() {
            b'M' => Self::Move,
            b'L' => Self::Line,
            b'H' => Self::Horizontal,
            b'V' => Self::Vertical,
            b'C' => Self::Cubic,
            b'Q' => Self::Quadratic,
            b'A' => Self::Arc,
            b'Z' => Self::Close,
            _ => return None,
        };
        Some((verb, letter.is_ascii_lowercase()))
    }
}
