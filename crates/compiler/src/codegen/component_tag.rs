//! A child component tag mounts the child with a props struct built
//! from the attributes; a literal converts with `into`, an expression
//! is passed raw, and `bind:name={signal}` passes the signal itself.

use super::allowances::Allowance;
use super::emit::{Emitter, Place};
use crate::CompileError;
use crate::ast::{AttrValue, ComponentTag};

impl Emitter<'_> {
    pub fn emit_component(
        &mut self,
        tag: &ComponentTag,
        place: &Place,
    ) -> Result<(), CompileError> {
        if !tag.children.is_empty() {
            return Err(self.source.error(
                tag.offset,
                format!("children of a component tag are deferred (they will become snippets): close `<{}>` with ` />`", tag.name),
            ));
        }
        let last = tag.name.rsplit("::").next().unwrap_or(&tag.name);
        let var = self.fresh(&last.to_lowercase());
        let binding = if place.roots {
            var.clone()
        } else {
            format!("_{var}")
        };
        self.line(format!(
            "let {binding} = {}::mount_before(dom, {}, {}, {}Props {{",
            tag.name,
            place.parent,
            place.before(),
            tag.name
        ));
        self.indent(1);
        for attribute in tag.attributes.iter() {
            let name = attribute
                .name
                .strip_prefix("bind:")
                .unwrap_or(&attribute.name);
            let value = match &attribute.value {
                AttrValue::Literal(value) => format!("{value:?}.into()"),
                AttrValue::Expr(expr) => self.raw(expr)?,
            };
            self.line(format!("{name}: {value},"));
        }
        self.allow(Allowance::NeedlessUpdate);
        self.line("..Default::default()");
        self.dedent(1);
        self.line("});");
        if place.roots {
            self.line(format!("roots.extend({var}.roots().iter().copied());"));
        }
        Ok(())
    }
}
