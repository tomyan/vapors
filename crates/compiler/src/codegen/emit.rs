//! The emitter: lines of Rust for the template, fresh variable names,
//! the scope of names that auto-unwrap; `clones` has the clones a `move`
//! closure needs of the script's plain bindings.

mod clones;
mod declared;
mod place;

pub(crate) use place::Place;

use std::collections::HashSet;

use super::script::ScriptInfo;
use super::syn_map::parse_expr;
use super::unwrap::unwrap_signals;
use crate::CompileError;
use crate::ast::{Expr, Node};
use crate::source::Source;

pub(crate) struct Emitter<'a> {
    pub source: &'a Source,
    pub script: &'a ScriptInfo,
    lines: Vec<String>,
    depth: usize,
    counter: usize,
    /// Digits in every node name's counter, so that no two names differ
    /// by an inserted digit (`text3`, `text13`), which clippy's
    /// `similar_names` reports; see `generate`.
    width: usize,
    scopes: Vec<Vec<String>>,
    generated: HashSet<String>,
    loop_bindings: Vec<(String, usize)>,
    /// Whether the component has a `<style>`: each element it creates
    /// is then tagged with its scope.
    pub scoped: bool,
    /// The Dom type builder closures take: the component's `D`, or
    /// `dyn Dom` inside a snippet (`vapors_dom::Snippet` says why).
    pub dom_type: &'static str,
    /// What the components a tag may name declare.
    pub index: Option<&'a super::PropsIndex>,
    /// The snippets declared so far: a closure that reads one gets a
    /// clone of it, as of a script binding.
    pub snippets: Vec<super::snippet_list::LocalSnippet>,
}

impl<'a> Emitter<'a> {
    pub fn new(source: &'a Source, script: &'a ScriptInfo) -> Emitter<'a> {
        Emitter {
            source,
            script,
            lines: Vec::new(),
            depth: 0,
            counter: 0,
            width: 1,
            scopes: Vec::new(),
            generated: HashSet::new(),
            loop_bindings: Vec::new(),
            scoped: false,
            dom_type: "D",
            index: None,
            snippets: Vec::new(),
        }
    }

    pub fn indent(&mut self, levels: usize) {
        self.depth += levels;
    }

    pub fn dedent(&mut self, levels: usize) {
        self.depth -= levels;
    }

    pub fn line(&mut self, text: impl AsRef<str>) {
        self.lines
            .push(format!("{}{}", "    ".repeat(self.depth), text.as_ref()));
    }

    /// The lines emitted.
    pub fn finish(self) -> String {
        self.lines.iter().fold(String::new(), |mut text, line| {
            text.push_str(line);
            text.push('\n');
            text
        })
    }

    /// Numbers names with `width` digits, zero-padded.
    pub fn with_width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }

    /// Checks component tags against `index`.
    pub fn with_index(mut self, index: &'a super::PropsIndex) -> Self {
        self.index = Some(index);
        self
    }

    /// Tags each element with the component's scope when `scoped`.
    pub fn with_scope(mut self, scoped: bool) -> Self {
        self.scoped = scoped;
        self
    }

    /// How many names `fresh` has made.
    pub fn names_made(&self) -> usize {
        self.counter
    }

    /// A fresh variable name with the given stem.
    pub fn fresh(&mut self, stem: &str) -> String {
        let name = format!("{stem}{:0width$}", self.counter, width = self.width);
        self.counter += 1;
        self.reserve(&name);
        name
    }

    /// A list's snippets first (`snippet_list`), then the rest in order.
    pub fn emit_nodes(&mut self, nodes: &[Node], place: &Place) -> Result<(), CompileError> {
        let outer = self.snippets.len();
        self.emit_list_snippets(nodes)?;
        for node in nodes
            .iter()
            .filter(|node| !matches!(node, Node::Snippet(_)))
        {
            self.emit_node(node, place)?;
        }
        self.snippets.truncate(outer);
        Ok(())
    }

    fn emit_node(&mut self, node: &Node, place: &Place) -> Result<(), CompileError> {
        match node {
            Node::Element(element) => self.emit_element(element, place),
            Node::Component(tag) => self.emit_component(tag, place),
            Node::Text(text) => self.emit_text(text, place),
            Node::If(block) => self.emit_if(block, place),
            Node::For(block) => self.emit_for(block, place),
            Node::Snippet(snippet) => self.emit_snippet(snippet),
            Node::Render(render) => self.emit_render(render, place),
        }
    }

    /// Places a freshly created node and, among the roots, records it.
    pub fn attach(&mut self, var: &str, place: &Place) {
        self.position(var, place);
        if place.roots {
            self.line(format!("roots.push(Root::Node({var}));"));
        }
    }

    /// Places a node at the end of its parent, or before `before` among
    /// the roots, without recording it (a block's anchor is recorded
    /// through the block).
    pub fn position(&mut self, var: &str, place: &Place) {
        if place.roots {
            self.line(format!(
                "dom.borrow_mut().insert_before({}, {var}, before);",
                place.parent
            ));
        } else {
            self.line(format!("dom.borrow_mut().append({}, {var});", place.parent));
        }
    }

    /// Names that read as `name.get()` here: script signals and memos
    /// plus the loop bindings in scope, innermost last.
    pub fn unwrappable(&self) -> Vec<String> {
        let mut names = self.script.unwrappable.clone();
        for scope in &self.scopes {
            names.extend(scope.iter().cloned());
        }
        names
    }

    pub fn push_scope(&mut self, names: Vec<String>) {
        self.scopes.push(names);
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// The expression's code with signal names unwrapped.
    pub fn unwrapped(&self, expr: &Expr) -> Result<String, CompileError> {
        let mut parsed = parse_expr(self.source, expr)?;
        unwrap_signals(&mut parsed, &self.unwrappable());
        Ok(super::syn_map::render(&parsed))
    }

    /// The raw code of an expression, checked to parse.
    pub fn raw(&self, expr: &Expr) -> Result<String, CompileError> {
        parse_expr(self.source, expr)?;
        Ok(expr.code.clone())
    }
}
