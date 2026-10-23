//! Auto-unwrap in text holes: a bare signal or memo name becomes a
//! read, unless it is the receiver of an explicit signal method call.

use std::collections::HashSet;

use syn::visit::Visit;
use syn::visit_mut::VisitMut;

/// Methods a template may call on a signal explicitly; the receiver
/// is then left alone instead of becoming `name.get().method()`.
const SIGNAL_METHODS: &[&str] = &[
    "dispose",
    "get",
    "get_untracked",
    "info",
    "set",
    "try_get",
    "try_get_untracked",
    "try_set",
    "try_update",
    "try_update_untracked",
    "try_with",
    "try_with_untracked",
    "update",
    "update_untracked",
    "with",
    "with_untracked",
];

/// Rewrites every bare path naming one of `names` into `name.get()`.
pub(crate) fn unwrap_signals(expr: &mut syn::Expr, names: &[String]) {
    Unwrapper { names }.visit_expr_mut(expr);
}

struct Unwrapper<'a> {
    names: &'a [String],
}

impl VisitMut for Unwrapper<'_> {
    fn visit_expr_mut(&mut self, expr: &mut syn::Expr) {
        if let syn::Expr::MethodCall(call) = expr
            && self.is_signal(&call.receiver)
            && SIGNAL_METHODS.contains(&call.method.to_string().as_str())
        {
            for arg in &mut call.args {
                self.visit_expr_mut(arg);
            }
            return;
        }
        if self.is_signal(expr) {
            let ident = single_ident(expr).expect("checked by is_signal");
            *expr = syn::parse_quote!(#ident.get());
            return;
        }
        syn::visit_mut::visit_expr_mut(self, expr);
    }
}

impl Unwrapper<'_> {
    fn is_signal(&self, expr: &syn::Expr) -> bool {
        single_ident(expr).is_some_and(|ident| self.names.iter().any(|name| ident == name))
    }
}

/// The identifier of a bare single-segment path, if that is what `expr` is.
fn single_ident(expr: &syn::Expr) -> Option<&syn::Ident> {
    let syn::Expr::Path(path) = expr else {
        return None;
    };
    if path.qself.is_some() || path.path.leading_colon.is_some() || path.path.segments.len() != 1 {
        return None;
    }
    let segment = &path.path.segments[0];
    segment.arguments.is_none().then_some(&segment.ident)
}

/// Every single-segment path identifier an expression mentions.
pub(crate) fn identifiers(expr: &syn::Expr) -> HashSet<String> {
    let mut collector = Collector::default();
    collector.visit_expr(expr);
    collector.names
}

#[derive(Default)]
struct Collector {
    names: HashSet<String>,
}

impl Visit<'_> for Collector {
    fn visit_expr_path(&mut self, path: &syn::ExprPath) {
        if let Some(ident) = single_ident(&syn::Expr::Path(path.clone())) {
            self.names.insert(ident.to_string());
        }
        syn::visit::visit_expr_path(self, path);
    }
}
