//! What the sheets' selectors use that widens a restyle (F2c S7): a
//! change to one element can restyle its later siblings when a sibling
//! combinator or a positional pseudo-class is used, its parent when
//! `:empty` is, and any element when `:has()` is.

use vapors_css::ast::Stylesheet;
use vapors_css::selector::{Combinator, Compound, PseudoClass, Selector};

/// The selector features of every sheet added so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Features {
    /// `+`, `~`, or a pseudo-class that depends on an element's place
    /// among its siblings (`:first-child`, `:nth-of-type()`, ...).
    pub(crate) siblings: bool,
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
        self.siblings |= selector
            .combinators
            .iter()
            .any(|c| matches!(c, Combinator::NextSibling | Combinator::SubsequentSibling));
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
                | PseudoClass::NthLastOfType(_) => self.siblings = true,
                PseudoClass::Hover
                | PseudoClass::Focus
                | PseudoClass::FocusVisible
                | PseudoClass::FocusWithin
                | PseudoClass::Active
                | PseudoClass::Checked
                | PseudoClass::Disabled
                | PseudoClass::Enabled
                | PseudoClass::Root => {}
            }
        }
    }
}
