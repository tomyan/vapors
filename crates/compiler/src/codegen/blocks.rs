//! `{if}` and `{for}`: an anchor text node marks the position, and
//! `vapors_dom::blocks` is handed the condition or the keyed items and
//! a builder closure for each branch or row.

use super::emit::{Emitter, Place};
use crate::CompileError;
use crate::ast::{Expr, ForBlock, IfBlock, Node};

const BUILDER: &str = "move |dom: &Rc<RefCell<D>>, parent: NodeId, before: Option<NodeId>| {";

impl Emitter<'_> {
    pub fn emit_if(&mut self, block: &IfBlock, place: &Place) -> Result<(), CompileError> {
        let anchor = self.fresh("if_anchor");
        self.emit_anchor(&anchor, place);
        let condition = self.raw(&block.condition)?;
        self.line("vapors_dom::blocks::conditional(");
        self.indent(1);
        self.line(format!("dom, {}, {anchor},", place.parent));
        let clones = self.clones_for(&[&block.condition])?;
        self.emit_closure(&clones, &[format!("move || {condition},")]);
        self.emit_builder(&block.then, BUILDER)?;
        match &block.otherwise {
            Some(nodes) => self.emit_builder(nodes, BUILDER)?,
            None => self.line("|_dom, _parent, _before| Vec::new(),"),
        }
        self.dedent(1);
        self.line(");");
        Ok(())
    }

    pub fn emit_for(&mut self, block: &ForBlock, place: &Place) -> Result<(), CompileError> {
        let anchor = self.fresh("for_anchor");
        self.emit_anchor(&anchor, place);
        let (index, item) = self.loop_names(block)?;
        let iterable = self.raw(&block.iterable)?;
        let key = block.key.as_ref().map(|key| self.raw(key)).transpose()?;
        self.line("vapors_dom::blocks::keyed(");
        self.indent(1);
        self.line(format!("dom, {}, {anchor},", place.parent));
        let mut read_by_items = vec![&block.iterable];
        read_by_items.extend(block.key.as_ref());
        let clones = self.clones_for(&read_by_items)?;
        self.emit_closure(
            &clones,
            &[keyed_items(&iterable, &index, &item, key.as_deref())],
        );
        let signature = format!(
            "move |dom: &Rc<RefCell<D>>, parent: NodeId, before: Option<NodeId>, {index}: Signal<usize>, {item}: Signal<_>| {{"
        );
        self.push_scope(vec![index.trim_start_matches('_').to_string(), item]);
        let built = self.emit_builder(&block.children, &signature);
        self.pop_scope();
        built?;
        self.dedent(1);
        self.line(");");
        Ok(())
    }

    /// `(index, item)` enumerates; `item` alone gets `_index`.
    fn loop_names(&self, block: &ForBlock) -> Result<(String, String), CompileError> {
        let pattern = block.pattern.trim();
        let inner = pattern
            .strip_prefix('(')
            .and_then(|rest| rest.strip_suffix(')'));
        let names: Vec<&str> = match inner {
            Some(inner) => inner.split(',').map(str::trim).collect(),
            None => vec!["_index", pattern],
        };
        if names.len() == 2 && names.iter().all(|name| is_identifier(name)) {
            return Ok((names[0].to_string(), names[1].to_string()));
        }
        Err(self.source.error(
            block.offset,
            format!("`{{for {pattern} in ...}}`: the pattern is `item` or `(index, item)`, two plain names"),
        ))
    }

    fn emit_anchor(&mut self, anchor: &str, place: &Place) {
        self.line(format!(
            "let {anchor} = dom.borrow_mut().create_text(\"\");"
        ));
        self.attach(anchor, place);
    }

    /// A builder closure: the nodes of a branch or row, returned as roots.
    fn emit_builder(&mut self, nodes: &[Node], signature: &str) -> Result<(), CompileError> {
        let clones = self.clones_for(&expressions_in(nodes))?;
        let wrapped = !clones.is_empty();
        if wrapped {
            self.line("{");
            self.indent(1);
            self.emit_clones(&clones);
        }
        self.line(signature);
        self.indent(1);
        self.line("let mut roots = Vec::new();");
        self.emit_nodes(nodes, &Place::roots())?;
        self.line("roots");
        self.dedent(1);
        self.line("},");
        if wrapped {
            self.dedent(1);
            self.line("}");
        }
        Ok(())
    }

    /// A closure expression, inside a block with its clones when any.
    fn emit_closure(&mut self, clones: &[String], lines: &[String]) {
        if clones.is_empty() {
            for line in lines {
                self.line(line);
            }
            return;
        }
        self.line("{");
        self.indent(1);
        self.emit_clones(clones);
        for line in lines {
            self.line(line);
        }
        self.dedent(1);
        self.line("},");
    }
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// Every expression a subtree reads, for the clones its closure needs.
fn expressions_in(nodes: &[Node]) -> Vec<&Expr> {
    let mut found = Vec::new();
    for node in nodes {
        collect(node, &mut found);
    }
    found
}

fn collect<'a>(node: &'a Node, found: &mut Vec<&'a Expr>) {
    match node {
        Node::Element(element) => {
            found.extend(element.attributes.iter().filter_map(|a| a.value.expr()));
            element
                .children
                .iter()
                .for_each(|child| collect(child, found));
        }
        Node::Component(tag) => found.extend(tag.attributes.iter().filter_map(|a| a.value.expr())),
        Node::Text(text) => found.extend(text.parts.iter().filter_map(|part| match part {
            crate::ast::Part::Expr(expr) => Some(expr),
            crate::ast::Part::Literal(_) => None,
        })),
        Node::If(block) => {
            found.push(&block.condition);
            block.then.iter().for_each(|child| collect(child, found));
            block
                .otherwise
                .iter()
                .flatten()
                .for_each(|child| collect(child, found));
        }
        Node::For(block) => {
            found.push(&block.iterable);
            found.extend(block.key.as_ref());
            block
                .children
                .iter()
                .for_each(|child| collect(child, found));
        }
        Node::Snippet(snippet) => snippet
            .children
            .iter()
            .for_each(|child| collect(child, found)),
        Node::Render(_) => {}
    }
}

/// The closure body listing `(key, item)` pairs. Without a key the
/// index is the key; `enumerate` appears only when the index is read.
fn keyed_items(iterable: &str, index: &str, item: &str, key: Option<&str>) -> String {
    match key {
        Some(key) if index.starts_with('_') => {
            format!("move || ({iterable}).into_iter().map(|{item}| (({key}), {item})).collect(),")
        }
        Some(key) => format!(
            "move || ({iterable}).into_iter().enumerate().map(|({index}, {item})| (({key}), {item})).collect(),"
        ),
        None => format!(
            "move || ({iterable}).into_iter().enumerate().map(|({index}, {item})| ({index}, {item})).collect(),"
        ),
    }
}
