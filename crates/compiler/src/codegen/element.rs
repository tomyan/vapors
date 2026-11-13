//! Elements: created from their `StyleKey` (`style_key`), attached,
//! then each attribute by its kind (`attribute_kind`): the key's own
//! (classes, directives, style, literals) need nothing more, an
//! expression is patched in an effect, `on<event>` is a listener,
//! `bind:` a listener plus a property effect, `bind:this` is set once
//! the element is whole, and the scope attribute is refused.

use super::allowances::shown_text;
use super::attribute_kind::{AttributeKind, kind};
use super::emit::{Emitter, Place};
use crate::CompileError;
use crate::ast::{AttrValue, Attribute, Attributes, Element, Expr};

impl Emitter<'_> {
    pub fn emit_element(&mut self, element: &Element, place: &Place) -> Result<(), CompileError> {
        let var = self.fresh(&element.tag);
        for attribute in &element.attributes.0 {
            super::attribute_refusals::refuse_misleading(self.source, attribute)?;
        }
        self.emit_keyed_create(element, &var)?;
        self.attach(&var, place);
        for attribute in element.attributes.iter() {
            self.emit_attribute(element, &var, attribute)?;
        }
        self.emit_class_directives(&element.attributes, &var)?;
        self.emit_nodes(&element.children, &Place::child_of(&var))?;
        // once the element is whole, so what reads the signal sees it built
        (element.attributes.iter())
            .filter(|attribute| kind(attribute) == AttributeKind::BindThis)
            .try_for_each(|attribute| self.emit_bind_this(&var, attribute))
    }

    /// What `attribute` needs after the element is created from its key.
    fn emit_attribute(
        &mut self,
        element: &Element,
        var: &str,
        attribute: &Attribute,
    ) -> Result<(), CompileError> {
        match kind(attribute) {
            AttributeKind::BindThis
            | AttributeKind::ClassDirective(_)
            | AttributeKind::Class
            | AttributeKind::Style
            | AttributeKind::Literal(_) => Ok(()),
            AttributeKind::Scope => Err(self.source.error(
                attribute.offset,
                format!(
                    "`{}` is reserved: it names the component's style scope, which mount sets on each element; use another attribute name",
                    crate::SCOPE_ATTRIBUTE
                ),
            )),
            AttributeKind::Listener(event) => self.emit_listener(var, event, attribute),
            AttributeKind::Binding(property) => {
                self.emit_binding(element, var, property, attribute)
            }
            AttributeKind::Expression(expr) => self.emit_expression_attribute(var, attribute, expr),
        }
    }

    /// An effect that sets `attribute` from its expression when it changes.
    fn emit_expression_attribute(
        &mut self,
        var: &str,
        attribute: &Attribute,
        expr: &Expr,
    ) -> Result<(), CompileError> {
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

/// The attribute called `name`, in any case, as HTML compares
/// attribute names.
pub(super) fn attribute_named<'e>(attributes: &'e Attributes, name: &str) -> Option<&'e Attribute> {
    attributes
        .iter()
        .find(|attribute| attribute.name.eq_ignore_ascii_case(name))
}
