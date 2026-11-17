//! Which longhands a declaration reached: one declared on the element
//! (any winner, `initial` and `unset` included), or, for an inherited
//! property, on an ancestor. A longhand no declaration reached computes
//! to its initial value, so a reader that compares values against their
//! initials (vapors-elements' `check_honoured`) skips it without
//! reading the value.

use std::sync::OnceLock;

use super::ComputedStyle;
use crate::Position;
use crate::property::{PROPERTIES, PropertyId};

/// One bit per longhand, in table order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Declared(u128);

/// The longhands `Declared` holds a bit for.
pub(crate) const BITS: usize = u128::BITS as usize;

impl Declared {
    /// The longhands with a winning declaration (`positions`, the
    /// computed style's, so the two answers are one), and those the
    /// parent declares among the inherited properties.
    pub(crate) fn of(
        positions: &[Option<(usize, Position)>],
        parent: Option<&ComputedStyle>,
    ) -> Self {
        let inherited = inherited_bits();
        let own = positions
            .iter()
            .enumerate()
            .filter(|(_, position)| position.is_some())
            .fold(0u128, |bits, (index, _)| bits | bit(index));
        Self(own | parent.map_or(0, |p| p.declared.0 & inherited))
    }

    /// Whether a declaration reached `id`.
    #[must_use]
    pub fn has(self, id: PropertyId) -> bool {
        self.0 & bit(id.index()) != 0
    }

    /// The longhands a declaration reached, in table order: a reader
    /// visits these rather than every longhand.
    pub fn ids(self) -> impl Iterator<Item = PropertyId> {
        let mut bits = self.0;
        std::iter::from_fn(move || {
            if bits == 0 {
                return None;
            }
            let index = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            Some(PropertyId::at(index))
        })
    }
}

/// The bits of the inherited properties, worked out once.
fn inherited_bits() -> u128 {
    static INHERITED: OnceLock<u128> = OnceLock::new();
    *INHERITED.get_or_init(|| {
        PropertyId::all()
            .filter(|id| id.inherited())
            .fold(0u128, |bits, id| bits | bit(id.index()))
    })
}

/// The bit of longhand `index`; one past the bits is refused, naming
/// the fix, before any shift (the table's size is also held by
/// `declared_tests.rs`, so `cargo test` refuses it first).
pub(crate) fn bit(index: usize) -> u128 {
    let shift = u32::try_from(index).ok().filter(|_| index < BITS);
    let Some(shift) = shift else {
        panic!(
            "longhand {index} of a table of {} has no bit in Declared's {BITS}; fix: widen \
             Declared in crates/css/src/cascade/declared.rs",
            PROPERTIES.len()
        );
    };
    1u128 << shift
}

#[cfg(test)]
#[path = "declared_tests.rs"]
mod tests;
