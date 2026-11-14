//! The expressions inside a macro call, so a `?` or a field chain in
//! `format!(.., x?.a.b)` is seen as the compiler sees it after
//! expansion. The tokens are read as a comma-separated list of
//! expressions; a macro whose arguments are not that (`matches!(x, pat)`,
//! a format string with named arguments is still expressions) gives up
//! its leading expressions, and one that starts with none gives nothing.

use syn::parse::{ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{Expr, Token};

/// The argument expressions of `mac`, as many as parse from the front.
pub fn macro_args(mac: &syn::Macro) -> Vec<Expr> {
    let all = Punctuated::<Expr, Token![,]>::parse_terminated;
    if let Ok(args) = all.parse2(mac.tokens.clone()) {
        return args.into_iter().collect();
    }
    leading_exprs.parse2(mac.tokens.clone()).unwrap_or_default()
}

/// Expressions from the front of `input` up to the first argument that
/// is not one; the rest is dropped.
fn leading_exprs(input: ParseStream) -> syn::Result<Vec<Expr>> {
    let mut found = Vec::new();
    // A failed parse may have moved the cursor, so each is tried on a fork.
    while input.fork().parse::<Expr>().is_ok() {
        found.push(input.parse::<Expr>()?);
        if input.parse::<Token![,]>().is_err() {
            break;
        }
    }
    let _rest: proc_macro2::TokenStream = input.parse()?;
    Ok(found)
}

#[cfg(test)]
#[path = "quality_macros_tests.rs"]
mod tests;
