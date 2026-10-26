//! Snippets (1-R1): `{render name()}` builds a snippet in place, and a
//! component tag's children become the child's `children` snippet, a
//! `vapors_dom::Snippet` closure built against `dyn Dom`.

use super::emit::{Emitter, Place};
use crate::CompileError;
use crate::ast::{Node, Render};

impl Emitter<'_> {
    /// `{render name()}`: the snippet's nodes at this place, among the
    /// roots when the place is.
    pub fn emit_render(&mut self, render: &Render, place: &Place) -> Result<(), CompileError> {
        if !render.args.trim().is_empty() {
            return Err(self.source.error(
                render.offset,
                format!("snippet arguments are deferred to 1-R1's second slice (R1b): render `{}()` without them", render.name),
            ));
        }
        let declared = self.script.bindings.contains(&render.name)
            || self
                .script
                .props
                .iter()
                .any(|prop| prop.name == render.name);
        if !declared {
            return Err(self.source.error(
                render.offset,
                format!("`{{render {0}()}}`: nothing named `{0}` is declared; declare it as a prop, `#[prop] let {0}: Snippet = Snippet::empty();`", render.name),
            ));
        }
        let call = format!(
            "{}.render(dom, {}, {})",
            render.name,
            place.parent,
            place.before()
        );
        if place.roots {
            self.line(format!("roots.extend({call});"));
        } else {
            self.line(format!("let _ = {call};"));
        }
        Ok(())
    }

    /// The children of a component tag as a `children` field of its
    /// props: a snippet built where the child renders it.
    pub fn emit_children(&mut self, children: &[Node], var: &str) -> Result<String, CompileError> {
        let local = format!("{var}_children");
        self.reserve(&local);
        self.line(format!("let {local} = vapors_dom::Snippet::new("));
        self.indent(1);
        let outer = std::mem::replace(&mut self.dom_type, "dyn Dom");
        let signature = self.builder_signature("");
        let built = self.emit_builder(children, &signature);
        self.dom_type = outer;
        self.dedent(1);
        self.line(");");
        built?;
        Ok(format!("children: {local},"))
    }
}
