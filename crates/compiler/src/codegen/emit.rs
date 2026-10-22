//! The emitter: lines of Rust for the template, fresh variable names,
//! the scope of names that auto-unwrap, and the clones a `move`
//! closure needs of the script's plain bindings.

use std::collections::{BTreeSet, HashSet};

use super::allowances::Allowance;
use super::script::ScriptInfo;
use super::syn_map::parse_expr;
use super::unwrap::{identifiers, unwrap_signals};
use crate::CompileError;
use crate::ast::{Expr, Node};
use crate::source::Source;

/// Where a node is placed: as a child of `parent`, and at the top
/// level of a component or block (`roots`), before `before`.
pub(crate) struct Place {
    pub parent: String,
    pub roots: bool,
}

impl Place {
    pub fn roots() -> Place {
        Place {
            parent: "parent".into(),
            roots: true,
        }
    }

    pub fn child_of(parent: &str) -> Place {
        Place {
            parent: parent.into(),
            roots: false,
        }
    }

    /// The `before` argument for a child component mount.
    pub fn before(&self) -> &'static str {
        if self.roots { "before" } else { "None" }
    }
}

pub(crate) struct Emitter<'a> {
    pub source: &'a Source,
    pub script: &'a ScriptInfo,
    lines: Vec<String>,
    depth: usize,
    counter: usize,
    scopes: Vec<Vec<String>>,
    allowances: BTreeSet<Allowance>,
}

impl<'a> Emitter<'a> {
    pub fn new(source: &'a Source, script: &'a ScriptInfo) -> Emitter<'a> {
        Emitter {
            source,
            script,
            lines: Vec::new(),
            depth: 0,
            counter: 0,
            scopes: Vec::new(),
            allowances: BTreeSet::new(),
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

    /// The lines emitted, and the lints the code they hold may trip.
    pub fn finish(self) -> (String, BTreeSet<Allowance>) {
        let body = self.lines.iter().fold(String::new(), |mut text, line| {
            text.push_str(line);
            text.push('\n');
            text
        });
        (body, self.allowances)
    }

    /// Records a lint the emitted code may trip; see `allowances`.
    pub fn allow(&mut self, allowance: Allowance) {
        self.allowances.insert(allowance);
    }

    /// A fresh variable name with the given stem.
    pub fn fresh(&mut self, stem: &str) -> String {
        let name = format!("{stem}{}", self.counter);
        self.counter += 1;
        name
    }

    pub fn emit_nodes(&mut self, nodes: &[Node], place: &Place) -> Result<(), CompileError> {
        for node in nodes {
            self.emit_node(node, place)?;
        }
        Ok(())
    }

    fn emit_node(&mut self, node: &Node, place: &Place) -> Result<(), CompileError> {
        match node {
            Node::Element(element) => self.emit_element(element, place),
            Node::Component(tag) => self.emit_component(tag, place),
            Node::Text(text) => self.emit_text(text, place),
            Node::If(block) => self.emit_if(block, place),
            Node::For(block) => self.emit_for(block, place),
            Node::Snippet(snippet) => Err(self.source.error(
                snippet.offset,
                "snippets are deferred: {snippet} and {render} parse but do not generate code yet",
            )),
            Node::Render(render) => Err(self.source.error(
                render.offset,
                "snippets are deferred: {snippet} and {render} parse but do not generate code yet",
            )),
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

    /// The script's plain bindings an expression mentions; a `move`
    /// closure that reads them gets a clone of each first.
    pub fn clones_for(&self, exprs: &[&Expr]) -> Result<Vec<String>, CompileError> {
        let mut mentioned = HashSet::new();
        for expr in exprs {
            mentioned.extend(identifiers(&parse_expr(self.source, expr)?));
        }
        let loop_names: HashSet<&String> = self.scopes.iter().flatten().collect();
        let mut clones: Vec<String> = self
            .script
            .bindings
            .iter()
            .filter(|name| mentioned.contains(*name) && !loop_names.contains(name))
            .cloned()
            .collect();
        clones.sort();
        Ok(clones)
    }

    /// `let name = name.clone();` for each clone.
    pub fn emit_clones(&mut self, clones: &[String]) {
        if !clones.is_empty() {
            self.allow(Allowance::CloneOnCopy);
        }
        for name in clones {
            self.line(format!("let {name} = {name}.clone();"));
        }
    }
}
