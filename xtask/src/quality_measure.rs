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
    /// Line of its `fn` keyword.
    pub line: usize,
    /// Lines of code in its body, as clippy's `too_many_lines` counts.
    pub code_lines: usize,
    /// clippy's cognitive complexity.
    pub complexity: usize,
    /// Under `#[cfg(test)]` or `#[test]`.
    pub test: bool,
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
    let attrs = match item {
        Item::Mod(m) => &m.attrs,
        Item::Fn(f) => &f.attrs,
        Item::Impl(i) => &i.attrs,
        Item::Struct(s) => &s.attrs,
        Item::Enum(e) => &e.attrs,
        Item::Const(c) => &c.attrs,
        Item::Static(s) => &s.attrs,
        Item::Trait(t) => &t.attrs,
        Item::Use(u) => &u.attrs,
        _ => return false,
    };
    attrs.iter().any(is_test_attr)
}

/// `#[test]` or `#[cfg(test)]`.
pub fn is_test_attr(attr: &syn::Attribute) -> bool {
    if attr.path().is_ident("test") {
        return true;
    }
    attr.path().is_ident("cfg")
        && attr
            .meta
            .require_list()
            .is_ok_and(|list| list.tokens.to_string() == "test")
}

struct Walker<'a> {
    index: LineIndex<'a>,
    impl_of: Vec<String>,
    in_test: Vec<bool>,
    functions: Vec<Function>,
}

impl Walker<'_> {
    fn in_test(&self) -> bool {
        self.in_test.last().copied().unwrap_or(false)
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
        });
    }
}

impl<'ast> Visit<'ast> for Walker<'_> {
    fn visit_item(&mut self, node: &'ast Item) {
        self.in_test.push(self.in_test() || is_test_item(node));
        visit::visit_item(self, node);
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
