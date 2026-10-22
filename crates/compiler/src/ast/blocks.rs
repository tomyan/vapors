//! Control-flow and snippet blocks.

use super::{Expr, Node};

/// `{if cond}...{else}...{/if}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfBlock {
    /// The condition, raw Rust.
    pub condition: Expr,
    /// The nodes when true.
    pub then: Vec<Node>,
    /// The nodes when false, if there is an `{else}`.
    pub otherwise: Option<Vec<Node>>,
    /// Byte offset of the `{if`.
    pub offset: usize,
}

/// `{for pattern in iterable key=expr}...{/for}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForBlock {
    /// The loop pattern, raw Rust (`item` or `(i, item)`).
    pub pattern: String,
    /// The iterable, raw Rust.
    pub iterable: Expr,
    /// The key expression, if any; without one, rows are keyed by index.
    pub key: Option<Expr>,
    /// The body.
    pub children: Vec<Node>,
    /// Byte offset of the `{for`.
    pub offset: usize,
}

/// `{snippet name(params)}...{/snippet}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snippet {
    /// The snippet's name.
    pub name: String,
    /// The parameter list including its parentheses.
    pub params: String,
    /// The body.
    pub children: Vec<Node>,
    /// Byte offset of the `{snippet`.
    pub offset: usize,
}

/// `{render name(args)}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Render {
    /// The snippet's name.
    pub name: String,
    /// The arguments without the parentheses.
    pub args: String,
    /// Byte offset of the `{render`.
    pub offset: usize,
}
