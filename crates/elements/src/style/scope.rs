//! A component's style, scoped to its own elements (decision D1). Every
//! compound of every selector also requires `data-wisp="<scope>"`, the
//! attribute a component's `mount` sets on each element it creates.
//! The subject compound requires it as an attribute selector, so a
//! scoped rule gains one attribute's specificity; every other compound,
//! and every compound inside `:is()`, `:not()`, `:where()` and `:has()`,
//! requires it inside `:where()`, which adds none. A compound holding
//! `:global(...)` is left unscoped. These are Svelte 5's rules.

use vapors_css::Position;
use vapors_css::ast::Stylesheet;
use vapors_css::selector::{
    AttributeOperator, AttributeSelector, Compound, PseudoClass, RelativeSelector, Selector,
};
use vapors_dom::SCOPE_ATTRIBUTE;

/// `sheet` with every selector scoped to `scope`.
pub(crate) fn scoped(mut sheet: Stylesheet, scope: &str) -> Stylesheet {
    for rule in &mut sheet.rules {
        scope_selector(&mut rule.selector, scope, true);
    }
    sheet
}

/// Scopes each compound; the subject's test counts for specificity
/// when `counted`.
fn scope_selector(selector: &mut Selector, scope: &str, counted: bool) {
    let last = selector.compounds.len().saturating_sub(1);
    for (index, compound) in selector.compounds.iter_mut().enumerate() {
        if is_global(compound) {
            continue;
        }
        scope_nested(compound, scope);
        if counted && index == last {
            compound.attributes.push(in_scope(scope));
        } else {
            compound.pseudo_classes.push(uncounted(scope));
        }
    }
}

/// Scopes the selectors inside a compound's pseudo-classes, none of
/// them counted: their specificity is the argument's, unchanged.
fn scope_nested(compound: &mut Compound, scope: &str) {
    for pseudo in &mut compound.pseudo_classes {
        match pseudo {
            PseudoClass::Not(list) | PseudoClass::Is(list) | PseudoClass::Where(list) => {
                for selector in list {
                    scope_selector(selector, scope, false);
                }
            }
            PseudoClass::Has(relatives) => {
                for RelativeSelector { selector, .. } in relatives {
                    scope_selector(selector, scope, false);
                }
            }
            _ => {}
        }
    }
}

fn is_global(compound: &Compound) -> bool {
    compound
        .pseudo_classes
        .iter()
        .any(|pseudo| matches!(pseudo, PseudoClass::Global(_)))
}

/// Where the first rule using `:global()` starts, for refusing it in a
/// sheet that is not a component's: there is no scope to leave.
pub(crate) fn first_global(sheet: &Stylesheet) -> Option<Position> {
    sheet
        .rules
        .iter()
        .find(|rule| rule.selector.compounds.iter().any(is_global))
        .map(|rule| rule.position)
}

fn in_scope(scope: &str) -> AttributeSelector {
    AttributeSelector {
        name: SCOPE_ATTRIBUTE.to_owned(),
        operator: AttributeOperator::Equals,
        value: scope.to_owned(),
        case_insensitive: false,
    }
}

/// `:where([data-wisp="<scope>"])`: in scope, with no specificity.
fn uncounted(scope: &str) -> PseudoClass {
    let compound = Compound {
        attributes: vec![in_scope(scope)],
        ..Compound::default()
    };
    PseudoClass::Where(vec![Selector {
        compounds: vec![compound],
        combinators: Vec::new(),
    }])
}
