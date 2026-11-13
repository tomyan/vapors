//! `class`, `class:` and `style` on a component tag
//! (`docs/plans/css-for-3d.md`, D1(b)): they are not props. The tag
//! gets a `StyleKey` as an element does (`style_key`), passed as the
//! generated prop `anchor: Some(&KEY)`, and the component creates its
//! one anchor element from it. Its `class:` directives toggle that
//! element, which the component returns from `anchor_element()`, in one
//! effect after the mount. A component in the same directory that
//! declares no `anchor` prop is refused at each such attribute; any
//! other component without the field fails in rustc at the props
//! struct, naming `anchor`.

use super::attribute_refusals::refuse_string_styling;
use super::element_class::is_class_directive;
use super::emit::Emitter;
use super::props_index::Declared;
use super::style_key::{Keyed, carried_by_key};
use crate::CompileError;
use crate::ast::{Attribute, ComponentTag};

/// The prop that carries the tag's key.
const ANCHOR_PROP: &str = "anchor";

/// Whether `attribute` styles the component's anchor element rather
/// than passing a prop.
pub(super) fn styles_anchor(attribute: &Attribute) -> bool {
    is_class_directive(attribute) || carried_by_key(&attribute.name)
}

impl Emitter<'_> {
    /// The props struct's `anchor` field when the tag carries a styling
    /// attribute, after adding the tag's key to the file's statics.
    pub(super) fn anchor_field(
        &mut self,
        tag: &ComponentTag,
        var: &str,
    ) -> Result<Option<String>, CompileError> {
        if !tag.attributes.iter().any(styles_anchor) {
            return Ok(None);
        }
        for attribute in tag.attributes.iter() {
            refuse_string_styling(self.source, attribute)?;
        }
        let key = self.key_static(&Keyed::component(tag), var)?;
        Ok(Some(format!("{ANCHOR_PROP}: Some(&{key}),")))
    }

    /// The effect toggling the mounted component's anchor element, when
    /// the tag has `class:` directives; `binding` holds the component.
    pub(super) fn emit_anchor_toggles(
        &mut self,
        tag: &ComponentTag,
        binding: &str,
        var: &str,
    ) -> Result<(), CompileError> {
        if !tag.attributes.iter().any(is_class_directive) {
            return Ok(());
        }
        let anchor = format!("{var}_anchor");
        self.reserve(&anchor);
        self.line(format!("let {anchor} = {binding}.anchor_element();"));
        self.emit_class_directives(&tag.attributes, &anchor)
    }
}

/// Each styling attribute of `tag`, at its offset, refused with the
/// message, when `declared` has no `anchor` prop to take the key.
pub(super) fn anchor_refusals(tag: &ComponentTag, declared: &Declared) -> Vec<(usize, String)> {
    if declared.props.iter().any(|prop| prop == ANCHOR_PROP) {
        return Vec::new();
    }
    let refused = tag
        .attributes
        .iter()
        .filter(|attribute| styles_anchor(attribute));
    refused
        .map(|attribute| {
            let message = format!(
                "`<{}>` takes no `{}`: it has no anchor element ({} declares no `{ANCHOR_PROP}` prop); put it on an element inside {}",
                tag.name, attribute.name, declared.file, declared.file
            );
            (attribute.offset, message)
        })
        .collect()
}
