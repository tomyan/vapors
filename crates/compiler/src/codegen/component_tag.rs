//! A child component tag mounts the child with a props struct built
//! from the attributes; a literal converts with `into`, an expression
//! is passed raw, and `bind:name={signal}` passes the signal itself.
//! Expressions are evaluated into their own `let`s first, so the
//! allowance on the props literal covers only generated code. `class`,
//! `class:` and `style` style the component's anchor element instead
//! (`component_anchor`).

use super::allowances::Allowance;
use super::component_anchor::styles_anchor;
use super::element_class::is_class_directive;
use super::emit::{Emitter, Place};
use crate::CompileError;
use crate::ast::{AttrValue, ComponentTag, Node};

impl Emitter<'_> {
    pub fn emit_component(
        &mut self,
        tag: &ComponentTag,
        place: &Place,
    ) -> Result<(), CompileError> {
        if let Some(this) = tag
            .attributes
            .iter()
            .find(|attribute| attribute.name == "bind:this")
        {
            return Err(self.source.error(this.offset, format!(
                "`bind:this` binds an element's node, and <{}> is a component: bind an element inside the component, and pass the signal down as a prop",
                tag.name
            )));
        }
        self.check_props(tag);
        let last = tag.name.rsplit("::").next().unwrap_or(&tag.name);
        let var = self.fresh(&last.to_lowercase());
        self.reserve(&format!("{var}_props"));
        let mut fields = self.emit_prop_values(tag, &var)?;
        fields.extend(self.anchor_field(tag, &var)?);
        let outer = self.snippets.len();
        self.emit_list_snippets(&tag.children)?;
        for child in &tag.children {
            if let Node::Snippet(snippet) = child {
                fields.push(format!("{0}: {0}.clone(),", snippet.name));
            }
        }
        // the parser drops whitespace-only text, so `<Card> </Card>`
        // passes no children
        let rest: Vec<Node> = tag
            .children
            .iter()
            .filter(|child| !matches!(child, Node::Snippet(_)))
            .cloned()
            .collect();
        if !rest.is_empty() {
            fields.push(self.emit_children(&rest, &var)?);
        }
        self.snippets.truncate(outer);
        self.line(Allowance::NeedlessUpdate.attribute());
        self.line(format!("let {var}_props = {}Props {{", tag.name));
        self.indent(1);
        for field in fields {
            self.line(field);
        }
        self.line("..Default::default()");
        self.dedent(1);
        self.line("};");
        let binding = self.emit_mount(tag, &var, place);
        self.emit_anchor_toggles(tag, &binding, &var)
    }

    /// `let {var}_{prop} = expr;` for each expression attribute; returns
    /// the props struct's field lines.
    fn emit_prop_values(
        &mut self,
        tag: &ComponentTag,
        var: &str,
    ) -> Result<Vec<String>, CompileError> {
        let mut fields = Vec::new();
        let props = tag.attributes.iter().filter(|a| !styles_anchor(a));
        for attribute in props {
            let name = super::tag_props::prop_name(attribute);
            let value = match &attribute.value {
                AttrValue::Literal(value) => format!("{value:?}.into()"),
                AttrValue::Expr(expr) => {
                    let local = format!("{var}_{name}");
                    self.reserve(&local);
                    self.line(format!("let {local} = {};", self.prop_value(expr)?));
                    local
                }
            };
            fields.push(format!("{name}: {value},"));
        }
        Ok(fields)
    }

    /// Mounts the component; returns the binding that holds it.
    fn emit_mount(&mut self, tag: &ComponentTag, var: &str, place: &Place) -> String {
        // the anchor toggles read the binding after the mount
        let read = place.roots || tag.attributes.iter().any(is_class_directive);
        let binding = if read {
            var.to_string()
        } else {
            format!("_{var}")
        };
        self.line(format!(
            "let {binding} = {}::mount_before(dom, {}, {}, {var}_props).owned_by_parent();",
            tag.name,
            place.parent,
            place.before()
        ));
        if place.roots {
            self.line(format!(
                "roots.extend({var}.root_entries().iter().cloned());"
            ));
        }
        binding
    }
}

impl Emitter<'_> {
    /// An attribute's value as Rust: a local snippet named alone is
    /// cloned, so passing it as a prop leaves it for later renders
    /// (review-v66b); a snippet is an `Rc`, so the clone is cheap.
    fn prop_value(&self, expr: &crate::ast::Expr) -> Result<String, CompileError> {
        let code = self.raw(expr)?;
        let name = code.trim();
        if self.snippets.iter().any(|snippet| snippet.name == name) {
            return Ok(format!("{name}.clone()"));
        }
        Ok(code)
    }
}
