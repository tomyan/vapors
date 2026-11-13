//! Each template element's `vapors_dom::StyleKey`: a file-level
//! `static` named after the element's node (`KEY_DIV0` for `div0`)
//! holding what the template says about it (its site, scope, tag,
//! literal classes, `class:` directive names and literal `style`), and
//! the `create_keyed_element(&KEY_DIV0)` that builds the element from
//! it. The tree sets those attributes from the key, so nothing else
//! is emitted for them. A component tag carrying `class`, `class:` or
//! `style` gets a key the same way, which it passes to the component
//! (`component_anchor`).

use super::attribute_refusals::refuse_string_styling;
use super::element::attribute_named;
use super::emit::Emitter;
use crate::CompileError;
use crate::ast::{AttrValue, Attribute, Attributes, ComponentTag, Element};

/// What a key is made from: an element, whose key's tag is its own, or
/// a component tag, whose key's tag is the `ANCHOR_TAG` its props type
/// declares, so an alias (`Group as Section`) still keys a `group`.
pub(super) struct Keyed<'e> {
    /// The key's `tag` field, as Rust.
    pub(super) tag: String,
    /// `div` or `Mesh`, for messages.
    pub(super) name: &'e str,
    /// Where `class:` directives past the limit can go instead.
    pub(super) elsewhere: String,
    pub(super) attributes: &'e Attributes,
    pub(super) offset: usize,
}

impl<'e> Keyed<'e> {
    pub(super) fn element(element: &'e Element) -> Self {
        Self {
            tag: format!("{:?}", element.tag),
            name: &element.tag,
            elsewhere: "a child element".to_owned(),
            attributes: &element.attributes,
            offset: element.offset,
        }
    }

    pub(super) fn component(tag: &'e ComponentTag) -> Self {
        Self {
            tag: format!("{}Props::ANCHOR_TAG", tag.name),
            name: &tag.name,
            elsewhere: format!("an element inside <{}>", tag.name),
            attributes: &tag.attributes,
            offset: tag.offset,
        }
    }
}

/// Whether `name` is an attribute the key carries, so the element
/// emits no `set_attribute` for it.
pub(super) fn carried_by_key(name: &str) -> bool {
    name.eq_ignore_ascii_case("class") || name.eq_ignore_ascii_case("style")
}

impl Emitter<'_> {
    /// Emits `let {var} = ..create_keyed_element(&KEY)` for `element`,
    /// after adding its key to the file-level statics.
    pub(super) fn emit_keyed_create(
        &mut self,
        element: &Element,
        var: &str,
    ) -> Result<(), CompileError> {
        let name = self.key_static(&Keyed::element(element), var)?;
        self.line(format!(
            "let {var} = dom.borrow_mut().create_keyed_element(&{name});"
        ));
        Ok(())
    }

    /// Adds `keyed`'s key to the file-level statics as `KEY_{VAR}`, and
    /// returns that name.
    pub(super) fn key_static(&mut self, keyed: &Keyed, var: &str) -> Result<String, CompileError> {
        let name = format!("KEY_{}", var.to_ascii_uppercase());
        let key = self.style_key(keyed)?;
        self.keys
            .push(format!("static {name}: vapors_dom::StyleKey = {key};"));
        Ok(name)
    }

    fn style_key(&self, keyed: &Keyed) -> Result<String, CompileError> {
        let (line, column) = self.source.position(keyed.offset);
        // the package, as `SCOPE` names it, so two crates' `src/card.wisp` differ
        let place = format!("{}:{line}:{column}", self.source.path);
        let site = format!("concat!(env!(\"CARGO_PKG_NAME\"), \"/\", {place:?})");
        let scope = if self.scoped { "Some(SCOPE)" } else { "None" };
        let classes = quoted(&self.literal_classes(keyed.attributes)?);
        let toggles = quoted(&self.key_directives(keyed)?);
        let style = self.key_style(keyed)?;
        Ok(format!(
            "vapors_dom::StyleKey {{ site: {site}, scope: {scope}, tag: {}, classes: &[{classes}], toggles: &[{toggles}], style: {style} }}",
            keyed.tag
        ))
    }

    /// The literal `style` as Rust; `style={..}` is refused here, where
    /// it would otherwise be dropped.
    fn key_style(&self, keyed: &Keyed) -> Result<String, CompileError> {
        match attribute_named(keyed.attributes, "style") {
            Some(
                attribute @ Attribute {
                    value: AttrValue::Expr(_),
                    ..
                },
            ) => {
                refuse_string_styling(self.source, attribute)?;
                unreachable!("refuse_string_styling refuses every `style={{..}}`")
            }
            Some(Attribute {
                value: AttrValue::Literal(text),
                ..
            }) => Ok(format!("Some({text:?})")),
            None => Ok("None".to_owned()),
        }
    }
}

impl Emitter<'_> {
    /// `keyed`'s directive names, refusing more than
    /// [`crate::MAX_CLASS_DIRECTIVES`], naming the element.
    fn key_directives<'e>(&self, keyed: &Keyed<'e>) -> Result<Vec<&'e str>, CompileError> {
        let names = self.directive_names(keyed.attributes)?;
        if names.len() > crate::MAX_CLASS_DIRECTIVES {
            return Err(self.source.error(
                keyed.offset,
                format!(
                    "<{}> has {} `class:` directives; at most {} fit one element, so move some to {}",
                    keyed.name,
                    names.len(),
                    crate::MAX_CLASS_DIRECTIVES,
                    keyed.elsewhere
                ),
            ));
        }
        Ok(names)
    }
}

/// The file-level block of every element's key, for `shell::assemble`
/// (a comment on it would not survive prettyplease, which keeps only
/// doc comments; `StyleKey`'s own docs say what the fields are).
pub(super) fn key_statics(keys: &[String]) -> String {
    if keys.is_empty() {
        return String::new();
    }
    let mut out = keys.join("\n");
    out.push_str("\n\n");
    out
}

/// `"a", "b"` for a slice literal.
fn quoted(names: &[&str]) -> String {
    let each: Vec<String> = names.iter().map(|n| format!("{n:?}")).collect();
    each.join(", ")
}
