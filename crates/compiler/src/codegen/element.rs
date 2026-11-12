//! Elements: created from their `StyleKey` (`style_key`), attached,
//! then each attribute the key does not carry by kind: a literal set
//! once, an expression patched in an effect, `on<event>` a listener,
//! `bind:` a listener plus a property effect.

use super::allowances::shown_text;
use super::element_class::is_class_directive;
use super::emit::{Emitter, Place};
use super::style_key::carried_by_key;
use crate::CompileError;
use crate::ast::{AttrValue, Attribute, Element, Expr};

impl Emitter<'_> {
    pub fn emit_element(&mut self, element: &Element, place: &Place) -> Result<(), CompileError> {
        let var = self.fresh(&element.tag);
        for attribute in &element.attributes.0 {
            super::attribute_refusals::refuse_misleading(self.source, attribute)?;
        }
        self.emit_keyed_create(element, &var)?;
        self.attach(&var, place);
        let (this, rest): (Vec<&Attribute>, Vec<&Attribute>) = element
            .attributes
            .iter()
            .partition(|attribute| attribute.name == "bind:this");
        for attribute in rest {
            if is_class_directive(attribute) || carried_by_key(&attribute.name) {
                continue;
            }
            self.emit_attribute(element, &var, attribute)?;
        }
        self.emit_class_directives(element, &var)?;
        self.emit_nodes(&element.children, &Place::child_of(&var))?;
        // once the element is whole, so what reads the signal sees it built
        this.into_iter()
            .try_for_each(|attribute| self.emit_bind_this(&var, attribute))
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
                    "dom.borrow_mut().set_attribute_if_changed({var}, {:?}, &value);",
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

    /// The listener for a handler: a zero-argument script closure is
    /// called with no event, anything else with the event; either may
    /// return `()` or `Result<(), E>`, whose `Err` goes to the error
    /// report (`vapors_dom::handle`).
    fn handler_expr(&self, expr: &Expr) -> Result<String, CompileError> {
        let code = self.raw(expr)?;
        self.refuse_assigning_handler(expr, &code)?;
        Ok(match self.script.handlers.get(&code) {
            Some(0) => {
                format!("move |_event: &mut Event| vapors_dom::handle_without_event(&{code})")
            }
            _ => format!("move |event: &mut Event| vapors_dom::handle(event, &{code})"),
        })
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
