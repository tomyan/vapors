//! Attributes and their values.

use super::Expr;

/// Attributes in source order, each name at most once.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Attributes(pub Vec<Attribute>);

impl Attributes {
    /// The value of the attribute called `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&AttrValue> {
        self.0
            .iter()
            .find(|attribute| attribute.name == name)
            .map(|attribute| &attribute.value)
    }

    /// How many attributes there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The attributes in source order.
    pub fn iter(&self) -> impl Iterator<Item = &Attribute> {
        self.0.iter()
    }
}

impl std::ops::Index<usize> for Attributes {
    type Output = Attribute;
    fn index(&self, index: usize) -> &Attribute {
        &self.0[index]
    }
}

/// One attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// The name as written (`class`, `onclick`, `bind:value`).
    pub name: String,
    /// Its value.
    pub value: AttrValue,
    /// Byte offset of the name.
    pub offset: usize,
}

/// An attribute value: `name="literal"` or `name={expr}` (`{name}` is
/// shorthand for `name={name}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrValue {
    /// A quoted string.
    Literal(String),
    /// A Rust expression.
    Expr(Expr),
}

impl AttrValue {
    /// The string of a literal value.
    #[must_use]
    pub fn literal(&self) -> Option<&str> {
        match self {
            AttrValue::Literal(value) => Some(value),
            AttrValue::Expr(_) => None,
        }
    }

    /// The code of an expression value.
    #[must_use]
    pub fn expr_code(&self) -> Option<&str> {
        match self {
            AttrValue::Expr(expr) => Some(&expr.code),
            AttrValue::Literal(_) => None,
        }
    }
}
