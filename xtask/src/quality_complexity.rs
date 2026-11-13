//! A function's cognitive complexity, counted as clippy's
//! `cognitive_complexity` counts it (`clippy_lints/src/cognitive_complexity.rs`)
//! on the compiler's desugared tree: 1, plus one per `if`, per `match`
//! with more than one arm, per guarded arm; `while` and `for` desugar
//! to a loop around an `if` or a two-arm `match`, `?` to a two-arm
//! `match` holding a `return`; `let ... else` is no branch, its block
//! walked; closures are not entered, nested items are their own bodies. Every `return` (and so every `?`) is then taken
//! off again, all of them in a function returning `Result`, half
//! otherwise. The hard check is clippy's; this count is for the band
//! below it. Declared gaps, each checked against clippy on a probe
//! crate (2026-11-13): a closure is a body of its own to clippy and is
//! not measured here; a return type that is an alias of `Result`
//! (`JsResult`, `MapOutcome`) is read as not `Result`, so its `?`s
//! count half instead of nothing.

use syn::visit::{self, Visit};
use syn::{Pat, ReturnType, Type};

use crate::quality_macros::macro_args;

/// clippy's count for a body whose function returns `output`.
pub fn complexity(body: &syn::Block, output: &ReturnType) -> usize {
    let mut counter = Counter { cc: 1, returns: 0 };
    counter.visit_block(body);
    let adjust = if returns_result(output) {
        counter.returns
    } else {
        counter.returns / 2
    };
    // clippy: unreachable code full of `return`s must not go negative.
    if counter.cc >= adjust {
        counter.cc - adjust
    } else {
        counter.cc
    }
}

/// Whether the declared return type's last segment is `Result`; a
/// type alias hiding one is not seen, and is counted as clippy's
/// "not Result" would not be. None has been found on main.
fn returns_result(output: &ReturnType) -> bool {
    let ReturnType::Type(_, ty) = output else {
        return false;
    };
    let Type::Path(path) = &**ty else {
        return false;
    };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "Result")
}

struct Counter {
    cc: usize,
    returns: usize,
}

impl<'ast> Visit<'ast> for Counter {
    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        self.cc += 1;
        visit::visit_expr_if(self, node);
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        if node.arms.len() > 1 {
            self.cc += 1;
        }
        self.cc += node
            .arms
            .iter()
            .filter(|arm| matches!(arm.pat, Pat::Guard(_)))
            .count();
        visit::visit_expr_match(self, node);
    }

    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        self.cc += 1;
        visit::visit_expr_while(self, node);
    }

    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.cc += 1;
        visit::visit_expr_for_loop(self, node);
    }

    fn visit_expr_try(&mut self, node: &'ast syn::ExprTry) {
        self.cc += 1;
        self.returns += 1;
        visit::visit_expr_try(self, node);
    }

    fn visit_expr_return(&mut self, node: &'ast syn::ExprReturn) {
        self.returns += 1;
        visit::visit_expr_return(self, node);
    }

    /// Not entered: clippy walks the body without closures.
    fn visit_expr_closure(&mut self, _: &'ast syn::ExprClosure) {}

    /// Not entered: a nested item is its own body.
    fn visit_item(&mut self, _: &'ast syn::Item) {}

    /// A macro's expansion is counted as the compiler sees it: its
    /// argument expressions, plus the branches the standard macros add.
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        self.cc += macro_branches(node);
        for arg in macro_args(node) {
            self.visit_expr(&arg);
        }
    }
}

/// Branches a standard macro's expansion adds: `matches!` a two-arm
/// match; `assert!` and `assert_eq!`/`assert_ne!` an `if`; the
/// `debug_assert` forms an `if cfg!(debug_assertions)` around that.
fn macro_branches(node: &syn::Macro) -> usize {
    let Some(name) = node.path.segments.last() else {
        return 0;
    };
    match name.ident.to_string().as_str() {
        "matches" | "assert" | "assert_eq" | "assert_ne" => 1,
        "debug_assert" | "debug_assert_eq" | "debug_assert_ne" => 2,
        _ => 0,
    }
}

#[cfg(test)]
#[path = "quality_complexity_tests.rs"]
mod tests;
