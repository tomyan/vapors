//! Each template element's `vapors_dom::StyleKey`: a file-level
//! `static` named after the element's node (`KEY_DIV0` for `div0`)
//! holding what the template says about it (its site, scope, tag,
//! literal classes, `class:` directive names and literal `style`), and
//! the `create_keyed_element(&KEY_DIV0)` that builds the element from
//! it. The tree sets those attributes from the key, so nothing else
//! is emitted for them.

use super::element_class::class_attribute;
use super::emit::Emitter;
use crate::CompileError;
use crate::ast::{AttrValue, Element};

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
        let name = format!("KEY_{}", var.to_ascii_uppercase());
        let key = self.style_key(element)?;
        self.keys
            .push(format!("static {name}: vapors_dom::StyleKey = {key};"));
        self.line(format!(
            "let {var} = dom.borrow_mut().create_keyed_element(&{name});"
        ));
        Ok(())
    }

    fn style_key(&self, element: &Element) -> Result<String, CompileError> {
        let (line, column) = self.source.position(element.offset);
        let site = format!("{}:{line}:{column}", self.source.path);
        let scope = if self.scoped { "Some(SCOPE)" } else { "None" };
        let classes = quoted(&self.literal_classes(element)?);
        let toggles = quoted(&self.key_directives(element)?);
        let style = match class_attribute(element, "style").map(|a| &a.value) {
            Some(AttrValue::Literal(text)) => format!("Some({text:?})"),
            // `style={..}` is refused before this (attribute_refusals)
            Some(AttrValue::Expr(_)) | None => "None".to_owned(),
        };
        Ok(format!(
            "vapors_dom::StyleKey {{ site: {site:?}, scope: {scope}, tag: {:?}, classes: &[{classes}], toggles: &[{toggles}], style: {style} }}",
            element.tag
        ))
    }
}

impl Emitter<'_> {
    /// `element`'s directive names, refusing more than
    /// [`crate::MAX_CLASS_DIRECTIVES`], naming the element.
    fn key_directives<'e>(&self, element: &'e Element) -> Result<Vec<&'e str>, CompileError> {
        let names = self.directive_names(element)?;
        if names.len() > crate::MAX_CLASS_DIRECTIVES {
            return Err(self.source.error(
                element.offset,
                format!(
                    "<{}> has {} `class:` directives; at most {} fit one element, so move some to a child element",
                    element.tag,
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
