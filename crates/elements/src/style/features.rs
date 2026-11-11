//! What the sheets' selectors use that widens a restyle (F2c S7): a
//! change to one element can restyle its siblings when a sibling
//! combinator or a positional pseudo-class is used (which siblings:
//! `SiblingDeps`), its parent when `:empty` is, and any element when
//! `:has()` is.

use vapors_css::ast::Stylesheet;
use vapors_css::selector::{Compound, PseudoClass, Selector};

use super::sibling_deps::SiblingDeps;

/// The selector features of every sheet added so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Features {
    /// Which siblings of a changed element `+`, `~` and the positional
    /// pseudo-classes (`:first-child`, `:nth-of-type()`, ...) reach.
    pub(crate) siblings: SiblingDeps,
    /// `:empty`, which a text change can flip on the text's parent.
    pub(crate) empty: bool,
    /// `:has()`, which a change anywhere below an element can flip.
    pub(crate) has: bool,
}

impl Features {
    /// These features and `sheet`'s.
    pub(crate) fn with(mut self, sheet: &Stylesheet) -> Self {
        for rule in &sheet.rules {
            self.add_selector(&rule.selector);
        }
        self
    }

    fn add_selector(&mut self, selector: &Selector) {
        self.siblings.merge(SiblingDeps::of_selector(selector));
        for compound in &selector.compounds {
            self.add_compound(compound);
        }
    }

    fn add_compound(&mut self, compound: &Compound) {
        for pseudo in &compound.pseudo_classes {
            match pseudo {
                PseudoClass::Empty => self.empty = true,
                PseudoClass::Has(relatives) => {
                    self.has = true;
                    for relative in relatives {
                        self.add_selector(&relative.selector);
                    }
                }
                PseudoClass::Not(list)
                | PseudoClass::Is(list)
                | PseudoClass::Where(list)
                | PseudoClass::Global(list) => {
                    for selector in list {
                        self.add_selector(selector);
                    }
                }
                PseudoClass::FirstChild
                | PseudoClass::LastChild
                | PseudoClass::OnlyChild
                | PseudoClass::FirstOfType
                | PseudoClass::LastOfType
                | PseudoClass::OnlyOfType
                | PseudoClass::NthChild(_)
                | PseudoClass::NthLastChild(_)
                | PseudoClass::NthOfType(_)
                | PseudoClass::NthLastOfType(_)
                | PseudoClass::Hover
                | PseudoClass::Focus
                | PseudoClass::FocusVisible
                | PseudoClass::FocusWithin
                | PseudoClass::Active
                | PseudoClass::Checked
                | PseudoClass::Disabled
                | PseudoClass::Enabled
                | PseudoClass::Invalid
                | PseudoClass::Root => {}
            }
        }
    }
}
