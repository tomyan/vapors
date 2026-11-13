//! One Rust source file measured: each function's lines of code and
//! complexity, as clippy counts them, and its field chains. A file that
//! does not parse is refused naming the line: the measurement never
//! approximates a file it cannot read.

use syn::visit::{self, Visit};
use syn::{Item, Type};

use crate::quality_chains::{Chain, chains};
use crate::quality_complexity::complexity;
use crate::quality_lines::{LineIndex, code_lines};

/// One function with a body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    /// `name`, or `Type::name` in an impl or a trait.
    pub name: String,
    /// Line of its `fn` keyword, where both clippy lints point.
    pub line: usize,
    /// Lines of code in its body, as clippy's `too_many_lines` counts.
    pub code_lines: usize,
    /// clippy's cognitive complexity.
    pub complexity: usize,
    /// Under `#[cfg(test)]` or `#[test]`.
    pub test: bool,
    /// Lints allowed on it or an enclosing item, which clippy then says
    /// nothing about.
    pub allowed: Allowed,
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

/// What one file measures.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct FileMeasure {
    /// Its functions, in source order.
    pub functions: Vec<Function>,
    /// Its field chains outside test code.
    pub chains: Vec<Chain>,
}

/// Measure `text`, a whole Rust file.
///
/// # Errors
///
/// The parse error with its line, for a file that is not Rust.
pub fn measure_source(text: &str) -> Result<FileMeasure, String> {
    let file = syn::parse_file(text).map_err(|e| {
        let at = e.span().start();
        format!(
            "line {}: {e}; the quality measurement reads whole Rust files",
            at.line
        )
    })?;
    let mut walker = Walker {
        index: LineIndex::new(text),
        impl_of: Vec::new(),
        in_test: Vec::new(),
        allowed: vec![Allowed::with(Allowed::default(), &file.attrs)],
        functions: Vec::new(),
    };
    walker.visit_file(&file);
    Ok(FileMeasure {
        functions: walker.functions,
        chains: chains(&file),
    })
}

/// Whether an item is test-only: `#[cfg(test)]` on it, or a `#[test]` fn.
pub fn is_test_item(item: &Item) -> bool {
    item_attrs(item).iter().any(is_test_attr)
}

/// An item's outer attributes; none for the kinds that carry none here.
fn item_attrs(item: &Item) -> &[syn::Attribute] {
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

struct Walker<'a> {
    index: LineIndex<'a>,
    impl_of: Vec<String>,
    in_test: Vec<bool>,
    allowed: Vec<Allowed>,
    functions: Vec<Function>,
}

impl Walker<'_> {
    fn in_test(&self) -> bool {
        self.in_test.last().copied().unwrap_or(false)
    }

    fn allowed(&self) -> Allowed {
        self.allowed.last().copied().unwrap_or_default()
    }

    fn record(&mut self, sig: &syn::Signature, body: &syn::Block, attrs: &[syn::Attribute]) {
        let is_test_fn = attrs.iter().any(|a| a.path().is_ident("test"));
        let span = body.brace_token.span.join();
        let text = self.index.slice(span.start(), span.end());
        let name = match self.impl_of.last() {
            Some(ty) => format!("{ty}::{}", sig.ident),
            None => sig.ident.to_string(),
        };
        self.functions.push(Function {
            name,
            line: sig.fn_token.span.start().line,
            code_lines: code_lines(text),
            complexity: complexity(body, &sig.output),
            test: self.in_test() || is_test_fn || attrs.iter().any(is_test_attr),
            allowed: Allowed::with(self.allowed(), attrs),
        });
    }
}

impl<'ast> Visit<'ast> for Walker<'_> {
    fn visit_item(&mut self, node: &'ast Item) {
        self.in_test.push(self.in_test() || is_test_item(node));
        self.allowed
            .push(Allowed::with(self.allowed(), item_attrs(node)));
        visit::visit_item(self, node);
        self.allowed.pop();
        self.in_test.pop();
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        self.impl_of.push(type_name(&node.self_ty));
        visit::visit_item_impl(self, node);
        self.impl_of.pop();
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        self.impl_of.push(node.ident.to_string());
        visit::visit_item_trait(self, node);
        self.impl_of.pop();
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.record(&node.sig, &node.block, &node.attrs);
        visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.record(&node.sig, &node.block, &node.attrs);
        visit::visit_impl_item_fn(self, node);
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        if let Some(body) = &node.default {
            self.record(&node.sig, body, &node.attrs);
        }
        visit::visit_trait_item_fn(self, node);
    }
}

/// The last segment of a path type; `impl` for any other.
fn type_name(ty: &Type) -> String {
    match ty {
        Type::Path(path) => path
            .path
            .segments
            .last()
            .map_or_else(|| "impl".to_owned(), |s| s.ident.to_string()),
        Type::Reference(reference) => type_name(&reference.elem),
        _ => "impl".to_owned(),
    }
}

#[cfg(test)]
#[path = "quality_measure_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "quality_fixture_tests.rs"]
mod fixture_tests;
