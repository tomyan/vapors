//! Which siblings of an inserted or removed element a selector can
//! reach (splice.md (b2)): kept once per sheet added, read by the
//! restyle marks on insert and remove. Over-approximates, never under:
//! what cannot be placed widens to every child.

use vapors_css::selector::{Combinator, PseudoClass, Selector};

/// The siblings of a changed element whose matching a selector can
/// change, counted from that element among its parent's element
/// children.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // independent sets a caller unions
pub(crate) struct SiblingDeps {
    /// How many of the next siblings: one per `+`, one more for
    /// `:first-child` or `:only-child`.
    pub(crate) next: usize,
    /// The previous sibling: `:last-child`, `:only-child`.
    pub(crate) previous: bool,
    /// Every later sibling: `~`, `:nth-child()`, and the first, only
    /// and nth of-type forms.
    pub(crate) later: bool,
    /// Every earlier sibling: `:nth-last-child()`, and the last, only
    /// and nth-last of-type forms.
    pub(crate) earlier: bool,
    /// The reached siblings restyle with their subtrees: the use is in
    /// a group before the subject's.
    pub(crate) descendants: bool,
}

impl SiblingDeps {
    /// Whether any sibling is reached.
    pub(crate) fn any(self) -> bool {
        self != Self::default()
    }

    pub(crate) fn merge(&mut self, other: Self) {
        self.next = self.next.max(other.next);
        self.previous |= other.previous;
        self.later |= other.later;
        self.earlier |= other.earlier;
        self.descendants |= other.descendants;
    }

    /// The siblings `selector` can reach: the union over its groups of
    /// compounds joined by `+` or `~`.
    pub(crate) fn of_selector(selector: &Selector) -> Self {
        let mut deps = Self::default();
        let mut group = Group::default();
        for (i, compound) in selector.compounds.iter().enumerate() {
            group.add_compound(compound);
            match selector.combinators.get(i) {
                Some(Combinator::NextSibling) => group.plus += 1,
                Some(Combinator::SubsequentSibling) => group.tilde = true,
                Some(Combinator::Child | Combinator::Descendant) => {
                    deps.merge(std::mem::take(&mut group).resolve(true));
                }
                None => {}
            }
        }
        deps.merge(group.resolve(false));
        deps
    }
}

/// What one group of compounds uses, before it is placed.
#[derive(Default)]
#[allow(clippy::struct_excessive_bools)] // one flag per kind of use, resolved together
struct Group {
    plus: usize,
    tilde: bool,
    first: bool,
    previous: bool,
    later: bool,
    earlier: bool,
    /// What `:is()`, `:not()`, `:where()` and `:global()` selectors of
    /// several compounds reach (`of_selector` on each).
    nested: SiblingDeps,
}

impl Group {
    fn add_compound(&mut self, compound: &vapors_css::selector::Compound) {
        for pseudo in &compound.pseudo_classes {
            self.add_pseudo(pseudo);
        }
    }

    fn add_pseudo(&mut self, pseudo: &PseudoClass) {
        match pseudo {
            PseudoClass::FirstChild => self.first = true,
            PseudoClass::LastChild => self.previous = true,
            PseudoClass::OnlyChild => (self.first, self.previous) = (true, true),
            PseudoClass::NthChild(_) | PseudoClass::NthOfType(_) | PseudoClass::FirstOfType => {
                self.later = true;
            }
            PseudoClass::NthLastChild(_)
            | PseudoClass::NthLastOfType(_)
            | PseudoClass::LastOfType => {
                self.earlier = true;
            }
            PseudoClass::OnlyOfType => (self.later, self.earlier) = (true, true),
            PseudoClass::Not(list)
            | PseudoClass::Is(list)
            | PseudoClass::Where(list)
            | PseudoClass::Global(list) => self.add_nested(list),
            // `:has()` restyles everything (`Features::has`); `:empty`
            // the parent's subtree (`Features::empty`)
            PseudoClass::Has(_)
            | PseudoClass::Empty
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

    /// A nested selector of one compound uses what that compound uses;
    /// one with combinators reaches what it reaches on its own. This is
    /// the only place nested selectors are read (`Features` does not
    /// walk them for siblings).
    fn add_nested(&mut self, list: &[Selector]) {
        for selector in list {
            match selector.compounds.as_slice() {
                [compound] => self.add_compound(compound),
                _ => self.nested.merge(SiblingDeps::of_selector(selector)),
            }
        }
    }

    /// The siblings this group reaches from the changed element. A use
    /// before a sibling combinator is reached from the flipped sibling's
    /// later siblings: `previous` with `~` becomes every later sibling,
    /// `earlier` with any sibling combinator every child.
    fn resolve(self, descendants: bool) -> SiblingDeps {
        let combinators = self.plus > 0 || self.tilde;
        let uses = self.first || self.previous || self.later || self.earlier;
        let mut nested = self.nested;
        nested.descendants |= descendants && nested.any();
        if !combinators && !uses {
            return nested;
        }
        let mut deps = if self.earlier && combinators {
            SiblingDeps {
                later: true,
                earlier: true,
                descendants,
                ..SiblingDeps::default()
            }
        } else {
            SiblingDeps {
                next: self.plus + usize::from(self.first),
                previous: self.previous,
                later: self.tilde || self.later,
                earlier: self.earlier,
                descendants,
            }
        };
        deps.merge(nested);
        deps
    }
}

#[cfg(test)]
#[path = "sibling_deps_tests.rs"]
mod tests;
