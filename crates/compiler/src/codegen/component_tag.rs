//! A child component tag mounts the child with a props struct built
//! from the attributes; a literal converts with `into`, an expression
//! is passed raw, and `bind:name={signal}` passes the signal itself.
//! Expressions are evaluated into their own `let`s first, so the
//! allowance on the props literal covers only generated code.

use super::allowances::Allowance;
use super::emit::{Emitter, Place};
use crate::CompileError;
use crate::ast::{AttrValue, ComponentTag, Node};

impl Emitter<'_> {
    pub fn emit_component(
        &mut self,
        tag: &ComponentTag,
        place: &Place,
    ) -> Result<(), CompileError> {
        self.check_props(tag)?;
        let last = tag.name.rsplit("::").next().unwrap_or(&tag.name);
        let var = self.fresh(&last.to_lowercase());
        self.reserve(&format!("{var}_props"));
        let mut fields = self.emit_prop_values(tag, &var)?;
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
        self.emit_mount(tag, &var, place);
        Ok(())
    }

    /// `let {var}_{prop} = expr;` for each expression attribute; returns
    /// the props struct's field lines.
    fn emit_prop_values(
        &mut self,
        tag: &ComponentTag,
        var: &str,
    ) -> Result<Vec<String>, CompileError> {
        let mut fields = Vec::new();
        for attribute in tag.attributes.iter() {
            let name = attribute
                .name
                .strip_prefix("bind:")
                .unwrap_or(&attribute.name);
            let value = match &attribute.value {
                AttrValue::Literal(value) => format!("{value:?}.into()"),
                AttrValue::Expr(expr) => {
                    let local = format!("{var}_{name}");
                    self.reserve(&local);
                    self.line(format!("let {local} = {};", self.raw(expr)?));
                    local
                }
            };
            fields.push(format!("{name}: {value},"));
        }
        Ok(fields)
    }

    fn emit_mount(&mut self, tag: &ComponentTag, var: &str, place: &Place) {
        let binding = if place.roots {
            var.to_string()
        } else {
            format!("_{var}")
        };
        self.line(format!(
            "let {binding} = {}::mount_before(dom, {}, {}, {var}_props);",
            tag.name,
            place.parent,
            place.before()
        ));
        if place.roots {
            self.line(format!(
                "roots.extend({var}.root_entries().iter().cloned());"
            ));
        }
    }
}
