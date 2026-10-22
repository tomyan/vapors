//! The parsed stylesheet: rules, declarations and component values.

use crate::Position;
use crate::property::Longhand;
use crate::selector::Selector;
use crate::serialize;
use crate::token::{Token, TokenKind};

/// A parsed stylesheet. Selector lists are exploded: one [`Rule`] per
/// selector, each sharing the block's declarations, in source order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stylesheet {
    /// Style rules in source order, conditional blocks flattened.
    pub rules: Vec<Rule>,
    /// `@keyframes` blocks in source order.
    pub keyframes: Vec<Keyframes>,
}

/// One selector with its declarations and the conditions it sits under.
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    /// The selector's component values, as written.
    pub prelude: Vec<ComponentValue>,
    /// The parsed selector.
    pub selector: Selector,
    /// Enclosing `@media`, `@container` and `@supports` conditions,
    /// outermost first.
    pub conditions: Vec<Condition>,
    /// Declarations in source order.
    pub declarations: Vec<Declaration>,
    /// Where the selector starts.
    pub position: Position,
}

/// A condition an enclosing at-rule placed on a rule.
#[derive(Clone, Debug, PartialEq)]
pub enum Condition {
    /// `@media <query>`.
    Media(Vec<ComponentValue>),
    /// `@container <condition>`.
    Container(Vec<ComponentValue>),
    /// `@supports <condition>`.
    Supports(Vec<ComponentValue>),
}

/// `name: value [!important]`.
#[derive(Clone, Debug, PartialEq)]
pub struct Declaration {
    /// The property name, lowercased (custom properties keep their case
    /// after `--`).
    pub name: String,
    /// The value's component values, leading and trailing whitespace
    /// removed, `!important` removed.
    pub value: Vec<ComponentValue>,
    /// Whether `!important` was present.
    pub important: bool,
    /// The typed longhands this declaration sets; empty for a custom
    /// property.
    pub longhands: Vec<Longhand>,
    /// Where the property name starts.
    pub position: Position,
}

/// A `@keyframes` block.
#[derive(Clone, Debug, PartialEq)]
pub struct Keyframes {
    /// The animation name.
    pub name: String,
    /// Stops in source order; a selector list gives one stop per offset.
    pub stops: Vec<KeyframeStop>,
}

/// One stop of a `@keyframes` block.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyframeStop {
    /// 0.0 for `from`, 1.0 for `to`, else the percentage over 100.
    pub offset: f64,
    /// Declarations at this stop.
    pub declarations: Vec<Declaration>,
}

/// A component value: a token, a function with its arguments, or a
/// bracketed block (CSS Syntax Level 3 section 5).
#[derive(Clone, Debug, PartialEq)]
pub enum ComponentValue {
    /// A preserved token.
    Token(Token),
    /// `name(args)`.
    Function {
        /// The function name as written.
        name: String,
        /// The arguments, commas included as tokens.
        args: Vec<ComponentValue>,
        /// Where the name starts.
        position: Position,
    },
    /// `[...]`, `(...)` or `{...}`.
    Block {
        /// The opening token: `OpenSquare`, `OpenParen` or `OpenCurly`.
        open: TokenKind,
        /// The contents.
        contents: Vec<ComponentValue>,
        /// Where the opening bracket is.
        position: Position,
    },
}

impl ComponentValue {
    /// Where this value starts in the source.
    #[must_use]
    pub fn position(&self) -> Position {
        match self {
            Self::Token(t) => t.position,
            Self::Function { position, .. } | Self::Block { position, .. } => *position,
        }
    }

    /// Whether this is a whitespace token.
    #[must_use]
    pub fn is_whitespace(&self) -> bool {
        matches!(
            self,
            Self::Token(Token {
                kind: TokenKind::Whitespace,
                ..
            })
        )
    }
}

impl Rule {
    /// The selector as CSS text.
    #[must_use]
    pub fn selector_text(&self) -> String {
        serialize::component_values(&self.prelude)
    }

    /// The enclosing `@media` query text, if any.
    #[must_use]
    pub fn media_text(&self) -> Option<String> {
        self.conditions.iter().find_map(|c| match c {
            Condition::Media(v) => Some(serialize::component_values(v)),
            _ => None,
        })
    }

    /// The enclosing `@container` condition text, if any.
    #[must_use]
    pub fn container_text(&self) -> Option<String> {
        self.conditions.iter().find_map(|c| match c {
            Condition::Container(v) => Some(serialize::component_values(v)),
            _ => None,
        })
    }

    /// The enclosing `@supports` condition text, if any.
    #[must_use]
    pub fn supports_text(&self) -> Option<String> {
        self.conditions.iter().find_map(|c| match c {
            Condition::Supports(v) => Some(serialize::component_values(v)),
            _ => None,
        })
    }
}

impl Declaration {
    /// The value as CSS text.
    #[must_use]
    pub fn value_text(&self) -> String {
        serialize::component_values(&self.value)
    }
}

impl Stylesheet {
    /// The stylesheet as CSS text that parses back to an equal stylesheet.
    #[must_use]
    pub fn to_css(&self) -> String {
        serialize::stylesheet(self)
    }
}
