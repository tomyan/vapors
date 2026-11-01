//! A handler that assigns to a binding it captures needs `FnMut`, and
//! handlers are `Fn` (a listener may run again inside itself when it
//! dispatches, doc 03): found here so the compiler refuses it in the
//! `.wisp`, naming the binding, not rustc in the generated file. A
//! change through a method (`v.push(..)`) is left to rustc.

use std::collections::HashSet;

use syn::visit::Visit;

use super::emit::Emitter;
use super::syn_map::parse_expr;
use crate::CompileError;
use crate::ast::Expr;

impl Emitter<'_> {
    /// Refuses a handler, the script's or written inline, that assigns
    /// to a binding it captures.
    pub(super) fn refuse_assigning_handler(
        &self,
        expr: &Expr,
        code: &str,
    ) -> Result<(), CompileError> {
        let captured = match parse_expr(self.source, expr)? {
            syn::Expr::Closure(closure) => assigned_capture(&closure),
            _ => self.script.assigning.get(code.trim()).cloned(),
        };
        let Some(captured) = captured else {
            return Ok(());
        };
        Err(self.source.error(expr.offset, format!(
            "the handler `{code}` assigns to `{captured}`, which it captures: a handler is `Fn`, since a listener may run again inside itself when it dispatches; keep `{captured}` in a signal (`let {captured}: Signal<T> = Signal::new(..);` and `{captured}.update(..)`) or a `Cell`"
        )))
    }
}

/// The first binding `closure` assigns to (`=`, `+=` and the like, or
/// `&mut`) that it does not bind itself.
pub(crate) fn assigned_capture(closure: &syn::ExprClosure) -> Option<String> {
    let mut found = Found::default();
    found.visit_expr_closure(closure);
    found
        .assigned
        .into_iter()
        .find(|name| !found.bound.contains(name))
}

#[derive(Default)]
struct Found {
    /// Every name a pattern inside binds: over-counted, so a shadowed
    /// capture is left to rustc rather than refused wrongly.
    bound: HashSet<String>,
    assigned: Vec<String>,
}

impl<'ast> Visit<'ast> for Found {
    fn visit_pat_ident(&mut self, pat: &'ast syn::PatIdent) {
        self.bound.insert(pat.ident.to_string());
        syn::visit::visit_pat_ident(self, pat);
    }

    fn visit_expr_assign(&mut self, assign: &'ast syn::ExprAssign) {
        self.assigned.extend(root(&assign.left));
        syn::visit::visit_expr_assign(self, assign);
    }

    fn visit_expr_binary(&mut self, binary: &'ast syn::ExprBinary) {
        if is_compound_assign(&binary.op) {
            self.assigned.extend(root(&binary.left));
        }
        syn::visit::visit_expr_binary(self, binary);
    }

    fn visit_expr_reference(&mut self, reference: &'ast syn::ExprReference) {
        if reference.mutability.is_some() {
            self.assigned.extend(root(&reference.expr));
        }
        syn::visit::visit_expr_reference(self, reference);
    }
}

/// The binding a place expression names: `n`, `n.field`, `n[i]`.
fn root(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Path(path) => path.path.get_ident().map(ToString::to_string),
        syn::Expr::Field(field) => root(&field.base),
        syn::Expr::Index(index) => root(&index.expr),
        syn::Expr::Paren(paren) => root(&paren.expr),
        _ => None,
    }
}

fn is_compound_assign(op: &syn::BinOp) -> bool {
    use syn::BinOp::{
        AddAssign, BitAndAssign, BitOrAssign, BitXorAssign, DivAssign, MulAssign, RemAssign,
        ShlAssign, ShrAssign, SubAssign,
    };
    matches!(
        op,
        AddAssign(_)
            | SubAssign(_)
            | MulAssign(_)
            | DivAssign(_)
            | RemAssign(_)
            | BitXorAssign(_)
            | BitAndAssign(_)
            | BitOrAssign(_)
            | ShlAssign(_)
            | ShrAssign(_)
    )
}
