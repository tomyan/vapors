//! The two shapes `bind:this` takes, read from a declared type with
//! syn: `Signal<Option<NodeId>>` and `Signal<BTreeMap<K, NodeId>>`, each
//! type matched by its last path segment, so `vapors_signals::Signal<..>`
//! and `vapors_dom::NodeId` are taken as written.

/// Whether `ty` is `Signal<Option<NodeId>>`, each written with or
/// without its path: matched by the last segment of each.
pub(super) fn is_single(ty: &str) -> bool {
    signal_of(ty).is_some_and(|inner| shaped(&inner, "Option", 1))
}

/// Whether `ty` is `Signal<BTreeMap<K, NodeId>>`, with or without paths.
pub(super) fn is_keyed(ty: &str) -> bool {
    signal_of(ty).is_some_and(|inner| shaped(&inner, "BTreeMap", 2))
}

/// Whether `ty` is `name<.., NodeId>` with `count` type arguments.
fn shaped(ty: &syn::Type, name: &str, count: usize) -> bool {
    let Some((segment, arguments)) = last_segment(ty) else {
        return false;
    };
    segment == name
        && arguments.len() == count
        && arguments
            .last()
            .and_then(last_segment)
            .is_some_and(|(node, _)| node == "NodeId")
}

/// The `T` of `Signal<T>`, parsed from `ty`.
fn signal_of(ty: &str) -> Option<syn::Type> {
    let parsed: syn::Type = syn::parse_str(ty).ok()?;
    match last_segment(&parsed)? {
        (signal, mut arguments) if signal == "Signal" && arguments.len() == 1 => arguments.pop(),
        _ => None,
    }
}

/// `ty`'s last path segment's name and its type arguments.
fn last_segment(ty: &syn::Type) -> Option<(String, Vec<syn::Type>)> {
    let syn::Type::Path(path) = ty else {
        return None;
    };
    let last = path.path.segments.last()?;
    let syn::PathArguments::AngleBracketed(angled) = &last.arguments else {
        return Some((last.ident.to_string(), Vec::new()));
    };
    let arguments = angled
        .args
        .iter()
        .filter_map(|argument| match argument {
            syn::GenericArgument::Type(ty) => Some(ty.clone()),
            _ => None,
        })
        .collect();
    Some((last.ident.to_string(), arguments))
}
