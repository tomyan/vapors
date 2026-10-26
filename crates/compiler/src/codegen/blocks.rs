//! `{if}` and `{for}`: an anchor text node marks the position, and
//! `vapors_dom::blocks` is handed the condition or the keyed items and
//! a builder closure for each branch or row.

use super::emit::{Emitter, Place};
use crate::CompileError;
use crate::ast::{Expr, IfBlock, Node};

impl Emitter<'_> {
    pub fn emit_if(&mut self, block: &IfBlock, place: &Place) -> Result<(), CompileError> {
        let anchor = self.fresh("if_anchor");
        self.emit_anchor(&anchor, place);
        let condition = self.raw(&block.condition)?;
        let handle = self.open_block_call("conditional", place);
        self.line(format!("dom, {}, {anchor},", place.parent));
        let clones = self.clones_for(&[&block.condition])?;
        self.emit_closure(&clones, &[format!("move || {condition},")]);
        let builder = self.builder_signature("");
        self.emit_builder(&block.then, &builder)?;
        match &block.otherwise {
            Some(nodes) => self.emit_builder(nodes, &builder)?,
            None => self.line("|_dom, _parent, _before| Vec::new(),"),
        }
        self.close_block_call(handle);
        Ok(())
    }

    pub(super) fn emit_anchor(&mut self, anchor: &str, place: &Place) {
        self.line(format!(
            "let {anchor} = dom.borrow_mut().create_text(\"\");"
        ));
        self.position(anchor, place);
    }

    /// Starts a call into `vapors_dom::blocks`; among the roots its
    /// handle is kept, so the block's nodes go with whatever owns them.
    pub(super) fn open_block_call(&mut self, function: &str, place: &Place) -> Option<String> {
        let handle = place
            .roots
            .then(|| self.fresh(&format!("{function}_block")));
        match &handle {
            Some(handle) => self.line(format!("let {handle} = vapors_dom::blocks::{function}(")),
            None => self.line(format!("vapors_dom::blocks::{function}(")),
        }
        self.indent(1);
        handle
    }

    pub(super) fn close_block_call(&mut self, handle: Option<String>) {
        self.dedent(1);
        self.line(");");
        if let Some(handle) = handle {
            self.line(format!("roots.push(Root::Block({handle}));"));
        }
    }

    /// A builder closure: the nodes of a branch or row, returned as roots.
    pub(super) fn emit_builder(
        &mut self,
        nodes: &[Node],
        signature: &str,
    ) -> Result<(), CompileError> {
        let read = expressions_in(nodes);
        let clones = self.clones_for(&read.iter().collect::<Vec<_>>())?;
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
        // inside its clones' block the closure is the block's value; the
        // block, not the closure, is then the argument the comma ends
        self.line(if wrapped { "}" } else { "}," });
        if wrapped {
            self.dedent(1);
            self.line("},");
        }
        Ok(())
    }

    /// A closure expression, inside a block with its clones when any.
    pub(super) fn emit_closure(&mut self, clones: &[String], lines: &[String]) {
        if clones.is_empty() {
            for line in lines {
                self.line(line);
            }
            return;
        }
        self.line("{");
        self.indent(1);
        self.emit_clones(clones);
        // the closure is the block's value; the block takes the comma
        for line in lines {
            self.line(line.strip_suffix(',').unwrap_or(line));
        }
        self.dedent(1);
        self.line("},");
    }
}

/// A builder closure's opening, taking this emitter's Dom type, with
/// `extra` parameters after the place.
impl Emitter<'_> {
    pub(super) fn builder_signature(&self, extra: &str) -> String {
        format!(
            "move |dom: &Rc<RefCell<{}>>, parent: NodeId, before: Option<NodeId>{extra}| {{",
            self.dom_type
        )
    }
}

/// Every expression a subtree reads, for the clones its closure needs; a
/// `{render name()}` reads the snippet `name`.
fn expressions_in(nodes: &[Node]) -> Vec<Expr> {
    let mut found = Vec::new();
    for node in nodes {
        collect(node, &mut found);
    }
    found
}

fn collect(node: &Node, found: &mut Vec<Expr>) {
    match node {
        Node::Element(element) => {
            found.extend(
                element
                    .attributes
                    .iter()
                    .filter_map(|a| a.value.expr())
                    .cloned(),
            );
            element
                .children
                .iter()
                .for_each(|child| collect(child, found));
        }
        // a tag's children become a snippet closure inside this one,
        // which is `move`: what they read must be cloned here first, or
        // a later use of a non-Copy binding finds it moved
        Node::Component(tag) => {
            found.extend(
                tag.attributes
                    .iter()
                    .filter_map(|a| a.value.expr())
                    .cloned(),
            );
            tag.children.iter().for_each(|child| collect(child, found));
        }
        Node::Text(text) => found.extend(text.parts.iter().filter_map(|part| match part {
            crate::ast::Part::Expr(expr) => Some(expr.clone()),
            crate::ast::Part::Literal(_) => None,
        })),
        Node::If(block) => {
            found.push(block.condition.clone());
            block.then.iter().for_each(|child| collect(child, found));
            block
                .otherwise
                .iter()
                .flatten()
                .for_each(|child| collect(child, found));
        }
        Node::For(block) => {
            found.push(block.iterable.clone());
            found.extend(block.key.clone());
            block
                .children
                .iter()
                .for_each(|child| collect(child, found));
        }
        Node::Snippet(snippet) => snippet
            .children
            .iter()
            .for_each(|child| collect(child, found)),
        Node::Render(render) => found.push(Expr {
            code: render.name.clone(),
            offset: render.offset,
        }),
    }
}
