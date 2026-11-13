//! Build-time analysis of one rule against a template key
//! (`docs/plans/build-time-styles.md`, section 1): whether the rule can
//! **never** match an element the key makes, **always** matches every
//! one, or is **conditional** on run-time inputs, which are named. The
//! judgement reads only what the key fixes (tag, scope, literal classes,
//! literal attributes, literal `style`), with the matcher's own pieces
//! (`selector::matching`), so the compiler and the runtime cannot
//! disagree about what a selector means. Anything it cannot decide from
//! the fixed parts is conditional, which is exact because the runtime
//! tests those rules with the matcher.

mod compound;
mod facts;

pub use facts::KeyFacts;

use std::collections::BTreeSet;
use std::fmt;

use crate::ast::Rule;
use crate::selector::{Combinator, Selector};
use crate::tree::States;

/// The run-time inputs a conditional match depends on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Depends {
    /// `class:` directives whose state the match reads.
    pub toggles: BTreeSet<String>,
    /// State flags the match reads (`:hover`, `:focus`, ...).
    pub states: States,
    /// Attributes whose run-time value the match reads: every
    /// attribute the key does not fix (`literal_attributes`), set by
    /// an expression or at run time; `class` as text (`[class~=x]`).
    pub attributes: BTreeSet<String>,
    /// Compounds left of the subject, or `:root`: the mount site decides.
    pub ancestors: bool,
    /// Siblings or descendants: `+`, `~`, positional pseudo-classes,
    /// `:empty`, `:has()`.
    pub structure: bool,
    /// `@media`, `@supports` or `@container` conditions.
    pub conditions: bool,
}

/// What the analysis decided about a rule and a key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Match {
    /// No element of the key can match.
    Never,
    /// Every element of the key matches, in every state.
    Always,
    /// Decided at run time by the inputs named.
    Conditional(Depends),
}

impl Depends {
    fn union(mut self, other: Self) -> Self {
        self.toggles.extend(other.toggles);
        self.states = self.states.union(other.states);
        self.attributes.extend(other.attributes);
        self.ancestors |= other.ancestors;
        self.structure |= other.structure;
        self.conditions |= other.conditions;
        self
    }
}

impl Match {
    /// Both must hold.
    pub(crate) fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::Never, _) | (_, Self::Never) => Self::Never,
            (Self::Always, other) | (other, Self::Always) => other,
            (Self::Conditional(a), Self::Conditional(b)) => Self::Conditional(a.union(b)),
        }
    }

    /// Either may hold (`:is()`, `:not()`'s list).
    pub(crate) fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::Always, _) | (_, Self::Always) => Self::Always,
            (Self::Never, other) | (other, Self::Never) => other,
            (Self::Conditional(a), Self::Conditional(b)) => Self::Conditional(a.union(b)),
        }
    }

    /// The opposite (`:not()`).
    pub(crate) fn not(self) -> Self {
        match self {
            Self::Never => Self::Always,
            Self::Always => Self::Never,
            conditional @ Self::Conditional(_) => conditional,
        }
    }

    pub(crate) fn depends(depends: Depends) -> Self {
        Self::Conditional(depends)
    }
}

/// Classifies `rule` against the elements `facts` describes.
#[must_use]
pub fn classify(rule: &Rule, facts: &KeyFacts<'_>) -> Match {
    facts.check_shape();
    let conditions = if rule.conditions.is_empty() {
        Match::Always
    } else {
        Match::depends(Depends {
            conditions: true,
            ..Depends::default()
        })
    };
    selector(&rule.selector, facts).and(conditions)
}

/// A whole selector: its subject compound on the fixed parts, and
/// every compound left of it at run time (ancestors above a component's
/// root come from the mount site; siblings are structure).
pub(crate) fn selector(selector: &Selector, facts: &KeyFacts<'_>) -> Match {
    let subject = selector
        .compounds
        .last()
        .expect("a selector has a compound");
    let mut result = compound::judge(subject, facts);
    for combinator in &selector.combinators {
        let depends = match combinator {
            Combinator::Descendant | Combinator::Child => Depends {
                ancestors: true,
                ..Depends::default()
            },
            Combinator::NextSibling | Combinator::SubsequentSibling => Depends {
                structure: true,
                ..Depends::default()
            },
        };
        result = result.and(Match::depends(depends));
    }
    result
}

impl fmt::Display for Depends {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<String> = Vec::new();
        for toggle in &self.toggles {
            parts.push(format!("class:{toggle}"));
        }
        parts.extend(
            crate::selector::state_names(self.states)
                .into_iter()
                .map(str::to_owned),
        );
        for attribute in &self.attributes {
            parts.push(format!("[{attribute}]"));
        }
        for (flag, name) in [
            (self.ancestors, "ancestors"),
            (self.structure, "siblings or descendants"),
            (self.conditions, "@media/@supports/@container"),
        ] {
            if flag {
                parts.push(name.to_owned());
            }
        }
        write!(f, "{}", parts.join(", "))
    }
}
