//! Selectors: the parsed form, specificity, and matching against any tree
//! that implements [`Element`].

mod attribute;
mod element;
mod items;
mod matching;
mod nth;
mod parse;
mod pseudo;
mod specificity;
mod structural;

pub use element::Element;
pub use matching::matches;
pub use nth::Nth;
pub use specificity::Specificity;

use crate::ast::ComponentValue;
use crate::{Position, Result};

/// A complex selector: compounds joined by combinators. `combinators[i]`
/// joins `compounds[i]` to `compounds[i + 1]`; the subject is the last.
#[derive(Clone, Debug, PartialEq)]
pub struct Selector {
    /// At least one compound.
    pub compounds: Vec<Compound>,
    /// One fewer than the compounds.
    pub combinators: Vec<Combinator>,
}

/// How two compounds relate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Combinator {
    /// Whitespace: any ancestor.
    Descendant,
    /// `>`: the parent.
    Child,
    /// `+`: the previous element sibling.
    NextSibling,
    /// `~`: any previous element sibling.
    SubsequentSibling,
}

/// A compound selector: everything between two combinators. The universal
/// selector is the compound with nothing set.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Compound {
    /// Type selector, lowercased.
    pub tag: Option<String>,
    /// Id selector.
    pub id: Option<String>,
    /// Class selectors; all must be present.
    pub classes: Vec<String>,
    /// Attribute selectors; all must match.
    pub attributes: Vec<AttributeSelector>,
    /// Pseudo-classes in source order; all must match.
    pub pseudo_classes: Vec<PseudoClass>,
}

/// `[name]`, `[name=value]` and the other operators, with the `i` flag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttributeSelector {
    /// The attribute name as written.
    pub name: String,
    /// How the value is compared.
    pub operator: AttributeOperator,
    /// The value to compare to; empty for `Exists`.
    pub value: String,
    /// `[name=value i]`: compare ASCII case-insensitively.
    pub case_insensitive: bool,
}

/// Attribute selector operators, Selectors Level 4 section 6.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttributeOperator {
    /// `[name]`
    Exists,
    /// `=`
    Equals,
    /// `^=`
    Prefix,
    /// `$=`
    Suffix,
    /// `*=`
    Substring,
    /// `~=`: one of the whitespace-separated words.
    Includes,
    /// `|=`: equal, or a prefix followed by `-`.
    DashMatch,
}

/// The pseudo-classes the engine knows. Anything else is refused at parse.
#[derive(Clone, Debug, PartialEq)]
pub enum PseudoClass {
    /// `:hover`
    Hover,
    /// `:focus`
    Focus,
    /// `:focus-visible`
    FocusVisible,
    /// `:focus-within`
    FocusWithin,
    /// `:active`
    Active,
    /// `:checked`
    Checked,
    /// `:disabled`
    Disabled,
    /// `:enabled`
    Enabled,
    /// `:empty`
    Empty,
    /// `:root`
    Root,
    /// `:first-child`
    FirstChild,
    /// `:last-child`
    LastChild,
    /// `:only-child`
    OnlyChild,
    /// `:first-of-type`
    FirstOfType,
    /// `:last-of-type`
    LastOfType,
    /// `:only-of-type`
    OnlyOfType,
    /// `:nth-child(An+B)`
    NthChild(Nth),
    /// `:nth-last-child(An+B)`
    NthLastChild(Nth),
    /// `:nth-of-type(An+B)`
    NthOfType(Nth),
    /// `:nth-last-of-type(An+B)`
    NthLastOfType(Nth),
    /// `:not(list)`
    Not(Vec<Selector>),
    /// `:is(list)`
    Is(Vec<Selector>),
    /// `:where(list)`
    Where(Vec<Selector>),
    /// `:has(relative list)`
    Has(Vec<RelativeSelector>),
    /// `:global(list)`: matched and weighed as `:is(list)`. In a
    /// component's scoped style (vapors-elements) the compound holding
    /// it is left unscoped, as Svelte's is.
    Global(Vec<Selector>),
}

/// An argument of `:has()`: a selector anchored to the subject by a
/// combinator (`:has(> .a)`; descendant when none is written).
#[derive(Clone, Debug, PartialEq)]
pub struct RelativeSelector {
    /// How the anchor relates to the selector's first compound.
    pub combinator: Combinator,
    /// The selector after the combinator.
    pub selector: Selector,
}

/// Parse a selector list from text.
///
/// # Errors
/// The first construct refused: an unknown pseudo-class, a pseudo-element,
/// a dangling combinator or a malformed attribute selector.
pub fn parse_selector_list(text: &str) -> Result<Vec<Selector>> {
    let sheet = crate::parse::parse(&format!("{text} {{ }}"))?;
    Ok(sheet.rules.into_iter().map(|r| r.selector).collect())
}

/// Parse a selector list from a rule prelude.
pub(crate) fn parse_from_prelude(
    prelude: &[ComponentValue],
    at: Position,
) -> Result<Vec<Selector>> {
    parse::selector_list(prelude, at)
}
