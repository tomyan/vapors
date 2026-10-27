//! `bind:value` and `bind:checked` on an element: the signal sets the
//! property, the control's event sets the signal. A range's or number
//! field's value binds as a number; a number field's on `change`.

use super::emit::Emitter;
use crate::CompileError;
use crate::ast::{AttrValue, Attribute, Element};

const BIND_CONTROLS: &[&str] = &["input", "select", "textarea"];

impl Emitter<'_> {
    pub(super) fn emit_binding(
        &mut self,
        element: &Element,
        var: &str,
        property: &str,
        attribute: &Attribute,
    ) -> Result<(), CompileError> {
        let (event, read, wrap) = match property {
            "value" if self.numeric_input(element, attribute)? => {
                ("input", "number", "PropertyValue::Number")
            }
            "value" => ("input", "event.value().to_string()", "PropertyValue::Text"),
            "checked" => ("change", "checked", "PropertyValue::Bool"),
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
        // a numeric or checked binding takes what the control's own event
        // always carries; an event without it is a bug in the tree, and
        // panics naming it
        let set = match read {
            "number" => format!(
                "{signal}.set(event.value_as_number().expect(\"the {}'s {event} carries its number\"));",
                element.tag
            ),
            "checked" => format!(
                "{signal}.set(event.checked().expect(\"the {}'s {event} carries its state\"));",
                element.tag
            ),
            _ => format!("{signal}.set({read});"),
        };
        self.line(format!(
            "dom.borrow_mut().listen({var}, {event:?}, Box::new(move |event: &mut Event| {{ {set} }}));"
        ));
        Ok(())
    }

    /// Whether `element` is an `input` whose value binds as a number: a
    /// `type` of `range` or `number` (3-E3), written as a literal, since
    /// the binding's type is decided here.
    fn numeric_input(
        &self,
        element: &Element,
        attribute: &Attribute,
    ) -> Result<bool, CompileError> {
        if element.tag != "input" {
            return Ok(false);
        }
        match element.attributes.get("type") {
            Some(AttrValue::Literal(kind)) => Ok(matches!(
                kind.to_ascii_lowercase().as_str(),
                "range" | "number"
            )),
            Some(AttrValue::Expr(_)) => Err(self.source.error(
                attribute.offset,
                "`bind:value` on an input whose `type` is an expression: a range or number input binds its value as a number (Signal<f64>) and a text input as text (Signal<String>), so write type as a literal, such as type=\"range\"".to_owned(),
            )),
            None => Ok(false),
        }
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
}
