//! Which longhands a declaration reached: one declared on the element
//! (any winner, `initial` and `unset` included), or, for an inherited
//! property, on an ancestor. A longhand no declaration reached computes
//! to its initial value, so a reader that compares values against their
//! initials (vapors-elements' `check_honoured`) skips it without
//! reading the value.

use std::sync::OnceLock;

use super::ComputedStyle;
use super::collect::Winners;
use crate::property::{PROPERTIES, PropertyId};

/// One bit per longhand, in table order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Declared(u128);

impl Declared {
    /// The longhands `winners` declare, and those the parent declares
    /// among the inherited properties.
    pub(crate) fn of(winners: &Winners, parent: Option<&ComputedStyle>) -> Self {
        let own = winners
            .longhands
            .iter()
            .enumerate()
            .filter(|(_, winner)| winner.is_some())
            .fold(0u128, |bits, (index, _)| bits | (1 << index));
        let inherited = parent.map_or(0, |p| p.declared.0 & inherited_bits());
        Self(own | inherited)
    }

    /// Whether a declaration reached `id`.
    #[must_use]
    pub fn has(self, id: PropertyId) -> bool {
        self.0 & (1 << id.index()) != 0
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

/// The bits of the inherited properties, worked out once. The table
/// must fit the bits: a longer one is refused at the first restyle,
/// naming the fix.
fn inherited_bits() -> u128 {
    static BITS: OnceLock<u128> = OnceLock::new();
    *BITS.get_or_init(|| {
        assert!(
            PROPERTIES.len() <= 128,
            "the property table has {} longhands, more than Declared's 128 bits; fix: widen Declared in crates/css/src/cascade/declared.rs",
            PROPERTIES.len()
        );
        PropertyId::all()
            .filter(|id| id.inherited())
            .fold(0u128, |bits, id| bits | (1 << id.index()))
    })
}
