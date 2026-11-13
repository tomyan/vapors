//! Field-access chains, the Law-of-Demeter signal: `a.b.c.d` reaches
//! through `a` and `b` to read `d`. A chain's hops are its `.field`
//! steps in a row (`self.a.b.c` is 3), through parentheses and `*`;
//! a method call, a call or an index ends the chain, so iterator and
//! builder chains do not count, and what follows one starts a new chain.
//! Tuple fields (`.0`) are hops: a newtype's inside is another type's.
//! Test code is left out: `#[cfg(test)]` items and `#[test]` functions.

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, UnOp};

use crate::quality_macros::macro_args;
use crate::quality_measure::is_test_item;

/// One chain of field accesses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chain {
    /// Line of the chain's first token.
    pub line: usize,
    /// Field steps in a row.
    pub hops: usize,
    /// The chain as written, spaces removed, cut at 60 characters.
    pub text: String,
}

/// Every chain in `file` outside test code.
pub fn chains(file: &syn::File) -> Vec<Chain> {
    let mut finder = Finder { found: Vec::new() };
    finder.visit_file(file);
    finder.found
}

struct Finder {
    found: Vec<Chain>,
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_item(&mut self, node: &'ast syn::Item) {
        if !is_test_item(node) {
            visit::visit_item(self, node);
        }
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if !node.attrs.iter().any(|a| a.path().is_ident("test")) {
            visit::visit_impl_item_fn(self, node);
        }
    }

    fn visit_expr_field(&mut self, node: &'ast syn::ExprField) {
        let (hops, base) = count_hops(node);
        self.found.push(Chain {
            line: node.span().start().line,
            hops,
            text: shown(node),
        });
        // The chain is recorded once, at its outermost field; what is
        // below its base (a call's arguments, an index) is walked on.
        self.visit_expr(base);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        for arg in macro_args(node) {
            self.visit_expr(&arg);
        }
    }
}

/// The hops from `node` down to the first expression that is not a
/// field access, and that expression.
fn count_hops(node: &syn::ExprField) -> (usize, &Expr) {
    let mut hops = 1;
    let mut base: &Expr = &node.base;
    loop {
        match base {
            Expr::Field(field) => {
                hops += 1;
                base = &field.base;
            }
            Expr::Paren(paren) => base = &paren.expr,
            Expr::Unary(unary) if matches!(unary.op, UnOp::Deref(_)) => base = &unary.expr,
            _ => return (hops, base),
        }
    }
}

fn shown(node: &syn::ExprField) -> String {
    let text: String = node
        .to_token_stream()
        .to_string()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if text.chars().count() > 60 {
        let cut: String = text.chars().take(57).collect();
        return format!("{cut}...");
    }
    text
}

#[cfg(test)]
#[path = "quality_chains_tests.rs"]
mod tests;
