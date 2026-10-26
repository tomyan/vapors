//! The declaration a refused prop needs, written so it compiles when
//! pasted: a snippet's from its parameter types, a value's from its
//! literal when it is one, otherwise `()` for the author to replace.

use quote::ToTokens;
use syn::spanned::Spanned;

/// `#[prop] let name: Snippet<(A, B)> = Snippet::empty();` for a
/// snippet with parameters `params`, as written with parentheses.
pub(super) fn snippet(name: &str, params: &str) -> String {
    let inner = params.trim().trim_start_matches('(').trim_end_matches(')');
    let types: Vec<String> = syn::parse_str::<syn::ExprClosure>(&format!("|{inner}| ()"))
        .map(|closure| closure.inputs.iter().filter_map(param_type).collect())
        .unwrap_or_default();
    let ty = match types.len() {
        0 => "Snippet".to_owned(),
        1 => format!("Snippet<({},)>", types[0]),
        _ => format!("Snippet<({})>", types.join(", ")),
    };
    format!("#[prop] let {name}: {ty} = Snippet::empty();")
}

fn param_type(pat: &syn::Pat) -> Option<String> {
    let syn::Pat::Type(typed) = pat else {
        return None;
    };
    let ty = &typed.ty;
    Some(
        ty.span()
            .source_text()
            .unwrap_or_else(|| ty.to_token_stream().to_string()),
    )
}

/// The declaration for a `bind:` prop, the signal itself; its value
/// type is a placeholder.
pub(super) fn bound(name: &str) -> (String, bool) {
    (
        format!("#[prop] let {name}: Signal<()> = Signal::new(());"),
        true,
    )
}

/// The declaration for `name={code}`: typed by a literal, else `()`,
/// a placeholder.
pub(super) fn value(name: &str, code: &str) -> (String, bool) {
    let (ty, default) = match syn::parse_str::<syn::Lit>(code.trim()) {
        Ok(syn::Lit::Int(_)) => ("i32", "0"),
        Ok(syn::Lit::Float(_)) => ("f64", "0.0"),
        Ok(syn::Lit::Bool(_)) => ("bool", "false"),
        Ok(syn::Lit::Str(_)) => ("String", "String::new()"),
        _ => ("()", "()"),
    };
    (format!("#[prop] let {name}: {ty} = {default};"), ty == "()")
}
