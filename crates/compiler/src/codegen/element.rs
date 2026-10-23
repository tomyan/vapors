//! Elements: created, attached, then each attribute by kind: a literal
//! set once, an expression patched in an effect, `on<event>` a
//! listener, `bind:` a listener plus a property effect.

use super::allowances::shown_text;
use super::emit::{Emitter, Place};
use crate::CompileError;
use crate::ast::{AttrValue, Attribute, Element, Expr};

const BIND_CONTROLS: &[&str] = &["input", "select", "textarea"];

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

    fn emit_binding(
        &mut self,
        element: &Element,
        var: &str,
        property: &str,
        attribute: &Attribute,
    ) -> Result<(), CompileError> {
        let (event, read, wrap) = match property {
            "value" => ("input", "event.value().to_string()", "PropertyValue::Text"),
            "checked" => ("change", "event.checked()", "PropertyValue::Bool"),
            _ => return Err(self.source.error(attribute.offset, format!(
                "`bind:{property}` is not a binding: elements bind `bind:value` (input, textarea, select) or `bind:checked` (checkbox, radio)"
            ))),
        };
        let event = if element.tag == "select" {
            "change"
        } else {
            event
        };
        self.refuse_bind_conflicts(element, attribute, event)?;
        let AttrValue::Expr(signal) = &attribute.value else {
            return Err(self.source.error(
                attribute.offset,
                format!("`bind:{property}` takes a signal: write bind:{property}={{name}}"),
            ));
        };
        let signal = self.raw(signal)?;
        self.emit_effect(
            &[],
            &[format!(
                "dom.borrow_mut().set_property({var}, {property:?}, {wrap}({signal}.get()));"
            )],
        );
        self.line(format!(
            "dom.borrow_mut().listen({var}, {event:?}, Box::new(move |event: &mut Event| {{ {signal}.set({read}); }}));"
        ));
        Ok(())
    }

    fn refuse_bind_conflicts(
        &self,
        element: &Element,
        attribute: &Attribute,
        event: &str,
    ) -> Result<(), CompileError> {
        if !BIND_CONTROLS.contains(&element.tag.as_str()) {
            return Err(self.source.error(attribute.offset, format!(
                "`{}` binds a form control's value: use it on input, textarea or select, not <{}>",
                attribute.name, element.tag
            )));
        }
        let handler = format!("on{event}");
        if element.attributes.get(&handler).is_some() {
            return Err(self.source.error(attribute.offset, format!(
                "`{}` already listens for `{event}`; it cannot be combined with `{handler}`: keep one, and write the other's work inside it",
                attribute.name
            )));
        }
        Ok(())
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
