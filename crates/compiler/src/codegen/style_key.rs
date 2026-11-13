//! Each template element's `vapors_dom::StyleKey`: a file-level
//! `static` named after the element's node (`KEY_DIV0` for `div0`)
//! holding what the template says about it (its site, scope, tag,
//! literal classes, `class:` directive names and literal `style`), and
//! the `create_keyed_element(&KEY_DIV0)` that builds the element from
//! it. The tree sets those attributes from the key, so nothing else
//! is emitted for them. A component tag carrying `class`, `class:` or
//! `style` gets a key the same way, which it passes to the component
//! (`component_anchor`).

use super::element::attribute_named;
use super::emit::Emitter;
use crate::CompileError;
use crate::ast::{AttrValue, Attributes, ComponentTag, Element};

/// What a key is made from: an element, or a component tag, whose key's
/// tag is the component's name in lower case (`mesh` for `<Mesh>`).
pub(super) struct Keyed<'e> {
    pub(super) tag: String,
    pub(super) attributes: &'e Attributes,
    pub(super) offset: usize,
}

impl<'e> Keyed<'e> {
    pub(super) fn element(element: &'e Element) -> Self {
        Self {
            tag: element.tag.clone(),
            attributes: &element.attributes,
            offset: element.offset,
        }
    }

    pub(super) fn component(tag: &'e ComponentTag) -> Self {
        let last = tag.name.rsplit("::").next().unwrap_or(&tag.name);
        Self {
            tag: last.to_lowercase(),
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
        let style = match attribute_named(keyed.attributes, "style").map(|a| &a.value) {
            Some(AttrValue::Literal(text)) => format!("Some({text:?})"),
            // `style={..}` is refused before this (attribute_refusals)
            Some(AttrValue::Expr(_)) | None => "None".to_owned(),
        };
        Ok(format!(
            "vapors_dom::StyleKey {{ site: {site}, scope: {scope}, tag: {:?}, classes: &[{classes}], toggles: &[{toggles}], style: {style} }}",
            keyed.tag
        ))
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
                    "<{}> has {} `class:` directives; at most {} fit one element, so move some to a child element",
                    keyed.tag,
                    names.len(),
                    crate::MAX_CLASS_DIRECTIVES
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
