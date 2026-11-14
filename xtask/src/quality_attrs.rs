//! What attributes say to the measurement: test-only code (`#[test]`,
//! a `#[cfg(..)]` naming `test`), code a host clippy run may not compile
//! (any other `#[cfg(..)]`), and clippy's two counts allowed at the site
//! (`#[allow]` or `#[expect]` of `too_many_lines` or
//! `cognitive_complexity`), each on the item or an enclosing one.

use syn::Item;

/// Whether an item is test-only: `#[cfg(test)]` on it, or a `#[test]` fn.
pub fn is_test_item(item: &Item) -> bool {
    item_attrs(item).iter().any(is_test_attr)
}

/// An item's outer attributes; none for the kinds that carry none here.
pub fn item_attrs(item: &Item) -> &[syn::Attribute] {
    match item {
        Item::Mod(m) => &m.attrs,
        Item::Fn(f) => &f.attrs,
        Item::Impl(i) => &i.attrs,
        Item::Struct(s) => &s.attrs,
        Item::Enum(e) => &e.attrs,
        Item::Const(c) => &c.attrs,
        Item::Static(s) => &s.attrs,
        Item::Trait(t) => &t.attrs,
        Item::Use(u) => &u.attrs,
        _ => &[],
    }
}

/// `#[test]`, or a `#[cfg(...)]` naming `test` anywhere in it
/// (`cfg(test)`, `cfg(all(test, feature = "x"))`). `cfg_attr(test, ..)`
/// is not test-only code and is not matched.
pub fn is_test_attr(attr: &syn::Attribute) -> bool {
    if attr.path().is_ident("test") {
        return true;
    }
    attr.path().is_ident("cfg")
        && attr
            .meta
            .require_list()
            .is_ok_and(|list| names_test(list.tokens.clone()))
}

fn names_test(tokens: proc_macro2::TokenStream) -> bool {
    tokens.into_iter().any(|tree| match tree {
        proc_macro2::TokenTree::Ident(ident) => ident == "test",
        proc_macro2::TokenTree::Group(group) => names_test(group.stream()),
        _ => false,
    })
}

/// A `#[cfg(..)]` other than one naming `test`: a target or a feature.
pub fn is_gated(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && !is_test_attr(a))
}

/// Which of clippy's two counts are allowed at the site.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Allowed {
    /// `#[allow(clippy::too_many_lines)]` or `expect`.
    pub lines: bool,
    /// `#[allow(clippy::cognitive_complexity)]` or `expect`.
    pub complexity: bool,
}

impl Allowed {
    /// What `attrs` allow, on top of `outer`.
    pub fn with(outer: Self, attrs: &[syn::Attribute]) -> Self {
        let names: String = attrs
            .iter()
            .filter(|a| a.path().is_ident("allow") || a.path().is_ident("expect"))
            .filter_map(|a| a.meta.require_list().ok())
            .map(|list| list.tokens.to_string())
            .collect();
        Self {
            lines: outer.lines || names.contains("too_many_lines"),
            complexity: outer.complexity || names.contains("cognitive_complexity"),
        }
    }
}

#[cfg(test)]
#[path = "quality_attrs_tests.rs"]
mod tests;
