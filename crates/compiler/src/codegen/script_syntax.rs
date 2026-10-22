//! Reading a `<script>` `let`: its name and type, whether the type
//! or initialiser is a `Signal` or `Memo`, and naming a signal after
//! its binding.

/// The bound name and its type annotation, for `let name` and `let name: T`.
pub(crate) fn binding(pat: &syn::Pat) -> (Option<String>, Option<syn::Type>) {
    match pat {
        syn::Pat::Ident(ident) => (Some(ident.ident.to_string()), None),
        syn::Pat::Type(typed) => match &*typed.pat {
            syn::Pat::Ident(ident) => (Some(ident.ident.to_string()), Some((*typed.ty).clone())),
            _ => (None, None),
        },
        _ => (None, None),
    }
}

/// `Signal` or `Memo` when the type is one of them.
pub(crate) fn reactive_kind(ty: &syn::Type) -> Option<&'static str> {
    let syn::Type::Path(path) = ty else {
        return None;
    };
    match path.path.segments.last()?.ident.to_string().as_str() {
        "Signal" => Some("Signal"),
        "Memo" => Some("Memo"),
        _ => None,
    }
}

/// `Signal` or `Memo` when the expression is a `X::new(...)` call.
pub(crate) fn call_of_new(expr: &syn::Expr) -> Option<&'static str> {
    let syn::Expr::Call(call) = expr else {
        return None;
    };
    let syn::Expr::Path(path) = &*call.func else {
        return None;
    };
    let segments = &path.path.segments;
    if segments.len() != 2 || segments[1].ident != "new" {
        return None;
    }
    match segments[0].ident.to_string().as_str() {
        "Signal" => Some("Signal"),
        "Memo" => Some("Memo"),
        _ => None,
    }
}

/// `Signal::new(x)` becomes `Signal::named("name", x)` so refusals name
/// the signal; any other initialiser is kept.
pub(crate) fn named(init: syn::Expr, name: &str) -> syn::Expr {
    if call_of_new(&init).is_none() {
        return init;
    }
    let syn::Expr::Call(mut call) = init else {
        unreachable!("call_of_new matched a call");
    };
    if let syn::Expr::Path(path) = &mut *call.func {
        path.path.segments[1].ident = syn::Ident::new("named", path.path.segments[1].ident.span());
    }
    let name_literal = syn::LitStr::new(name, proc_macro2::Span::call_site());
    call.args.insert(0, syn::parse_quote!(#name_literal));
    syn::Expr::Call(call)
}
