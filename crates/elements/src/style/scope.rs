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

/// Scopes each compound but a `:global` one. When `counted`, the last
/// scoped compound's test counts for specificity, so a rule whose
/// subject is `:global(..)` still gains one attribute (Svelte 5).
fn scope_selector(selector: &mut Selector, scope: &str, counted: bool) {
    let counted_at = counted
        .then(|| selector.compounds.iter().rposition(|c| !is_global(c)))
        .flatten();
    for (index, compound) in selector.compounds.iter_mut().enumerate() {
        if is_global(compound) {
            continue;
        }
        scope_nested(compound, scope);
        if counted_at == Some(index) {
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

/// Where the first rule using `:global()` anywhere starts, for refusing
/// it in a sheet that is not a component's: there is no scope to leave.
pub(crate) fn first_global(sheet: &Stylesheet) -> Option<Position> {
    sheet
        .rules
        .iter()
        .find(|rule| holds_global(&rule.selector))
        .map(|rule| rule.position)
}

/// Where the first rule with a `:global()` inside `:is()`, `:not()`,
/// `:where()` or `:has()` starts. Only a top-level compound can be left
/// unscoped, so one there is refused, not ignored.
pub(crate) fn first_nested_global(sheet: &Stylesheet) -> Option<Position> {
    sheet
        .rules
        .iter()
        .find(|rule| {
            rule.selector
                .compounds
                .iter()
                .any(|c| arguments(c).any(holds_global))
        })
        .map(|rule| rule.position)
}

fn holds_global(selector: &Selector) -> bool {
    selector
        .compounds
        .iter()
        .any(|c| is_global(c) || arguments(c).any(holds_global))
}

/// The selectors inside a compound's `:is()`, `:not()`, `:where()` and
/// `:has()`.
fn arguments(compound: &Compound) -> impl Iterator<Item = &Selector> {
    compound
        .pseudo_classes
        .iter()
        .flat_map(|pseudo| match pseudo {
            PseudoClass::Not(list) | PseudoClass::Is(list) | PseudoClass::Where(list) => {
                list.iter().collect::<Vec<_>>()
            }
            PseudoClass::Has(relatives) => relatives.iter().map(|r| &r.selector).collect(),
            _ => Vec::new(),
        })
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
