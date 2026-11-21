//! What a `style:` binding does to the analysis of its element: the
//! bound longhand's winner is variable, an inline value only the
//! running page knows (decision 2026-11-20-style-bindings, 5).

use vapors_css::analysis::{Constancy, Winner};
use vapors_css::property::PropertyId;

use super::ElementFacts;

/// The longhand indices an element's `style:` bindings set.
pub(super) fn bound_longhands(facts: &ElementFacts) -> Vec<usize> {
    let names = facts.bound.iter().map(|property| property.css_name());
    let ids =
        names.map(|name| PropertyId::from_name(name).expect("a bindable property is a longhand"));
    ids.map(PropertyId::index).collect()
}

/// A bound property's winner: an inline value only the running page
/// knows beats whatever the analysis found, which stays as the base the
/// runtime enters with the element's own value above it.
fn bound_winner(found: Winner) -> Winner {
    match found {
        Winner::Constant { place, .. } => Winner::Variable {
            base: Some(place),
            overriders: Vec::new(),
        },
        Winner::Undeclared { .. } => Winner::Variable {
            base: None,
            overriders: Vec::new(),
        },
        variable @ Winner::Variable { .. } => variable,
    }
}

/// Makes each of the `bound` longhands' winners variable.
pub(super) fn bind_winners(longhands: &mut [Winner], bound: &[usize]) {
    for index in bound {
        let found = std::mem::replace(
            &mut longhands[*index],
            Winner::Undeclared {
                value: Constancy::Constant,
            },
        );
        longhands[*index] = bound_winner(found);
    }
}
