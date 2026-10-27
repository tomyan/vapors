//! Elements: created, attached, then each attribute by kind: a literal
//! set once, an expression patched in an effect, `on<event>` a
//! listener, `bind:` a listener plus a property effect.

use super::allowances::shown_text;
use super::emit::{Emitter, Place};
use crate::CompileError;
use crate::ast::{AttrValue, Attribute, Element, Expr};

impl Emitter<'_> {
    pub fn emit_element(&mut self, element: &Element, place: &Place) -> Result<(), CompileError> {
        let var = self.fresh(&element.tag);
        self.line(format!(
            "let {var} = dom.borrow_mut().create_element({:?});",
            element.tag
        ));
        self.attach(&var, place);
        if self.scoped {
            self.line(format!(
                "dom.borrow_mut().set_attribute({var}, SCOPE_ATTRIBUTE, SCOPE);"
            ));
        }
        for attribute in element.attributes.iter() {
            self.emit_attribute(element, &var, attribute)?;
        }
        self.emit_nodes(&element.children, &Place::child_of(&var))
    }

    fn emit_attribute(
        &mut self,
        element: &Element,
        var: &str,
        attribute: &Attribute,
    ) -> Result<(), CompileError> {
        if attribute.name == crate::SCOPE_ATTRIBUTE {
            return Err(self.source.error(
                attribute.offset,
                format!(
                    "`{}` is reserved: it names the component's style scope, which mount sets on each element; use another attribute name",
                    crate::SCOPE_ATTRIBUTE
                ),
            ));
        }
        if let Some(event) = attribute
            .name
            .strip_prefix("on")
            .filter(|rest| !rest.is_empty())
        {
            return self.emit_listener(var, event, attribute);
        }
        if let Some(property) = attribute.name.strip_prefix("bind:") {
            return self.emit_binding(element, var, property, attribute);
        }
        match &attribute.value {
            AttrValue::Literal(value) => {
                self.line(format!(
                    "dom.borrow_mut().set_attribute({var}, {:?}, {value:?});",
                    attribute.name
                ));
                Ok(())
            }
            AttrValue::Expr(expr) => {
                let value = self.raw(expr)?;
                let clones = self.clones_for(&[expr])?;
                let mut body = shown_text(&value).to_vec();
                body.push(format!(
                    "dom.borrow_mut().set_attribute({var}, {:?}, &value);",
                    attribute.name
                ));
                self.emit_effect(&clones, &body);
                Ok(())
            }
        }
    }

    fn emit_listener(
        &mut self,
        var: &str,
        event: &str,
        attribute: &Attribute,
    ) -> Result<(), CompileError> {
        let AttrValue::Expr(expr) = &attribute.value else {
            return Err(self.source.error(
                attribute.offset,
                format!(
                    "event attributes take an expression: write {0}={{handler}}, not {0}=\"...\"",
                    attribute.name
                ),
            ));
        };
        let handler = self.handler_expr(expr)?;
        let clones = self.clones_for(&[expr])?;
        self.emit_wrapped(
            &clones,
            &format!("dom.borrow_mut().listen({var}, {event:?}, Box::new({handler}));"),
        );
        Ok(())
    }

    /// A zero-argument script closure is wrapped so it receives no
    /// event; anything else is passed as the listener itself.
    fn handler_expr(&self, expr: &Expr) -> Result<String, CompileError> {
        let code = self.raw(expr)?;
        match self.script.handlers.get(&code) {
            Some(0) => Ok(format!("move |_event: &mut Event| {{ {code}(); }}")),
            _ => Ok(code),
        }
    }

    /// A statement, inside a block with the clones when there are any.
    pub fn emit_wrapped(&mut self, clones: &[String], statement: &str) {
        if clones.is_empty() {
            self.line(statement);
            return;
        }
        self.line("{");
        self.indent(1);
        self.emit_clones(clones);
        self.line(statement);
        self.dedent(1);
        self.line("}");
    }
}
