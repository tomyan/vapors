//! `class`, `class:` and `style` on a component tag
//! (`docs/plans/css-for-3d.md`, D1(b)): they are not props. The tag
//! gets a `StyleKey` as an element does (`style_key`), passed as the
//! generated prop `anchor: Some(&KEY)`, and the component creates its
//! one anchor element from it. Its `class:` directives toggle that
//! element, which the component returns from `anchor_element()`, in one
//! effect after the mount. The key's tag is the props type's
//! `ANCHOR_TAG`. A component in the same directory, a `.wisp` one, is
//! refused at each such attribute; another crate's component without
//! the field fails in rustc at the props struct, naming `anchor`.

use super::attribute_kind::carried_by_key;
use super::element_class::is_class_directive;
use super::emit::Emitter;
use super::props_index::Declared;
use super::style_key::Keyed;
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
        self.refuse_anchor_attribute(tag)?;
        if !tag.attributes.iter().any(styles_anchor) {
            return Ok(None);
        }
        let key = self.key_static(&Keyed::component(tag), var)?;
        Ok(Some(format!("{ANCHOR_PROP}: Some(&{key}),")))
    }

    /// `anchor={..}` written on the tag: the key would not be the one
    /// the template's classes and style make, and could not be seen.
    fn refuse_anchor_attribute(&self, tag: &ComponentTag) -> Result<(), CompileError> {
        let Some(written) = tag.attributes.iter().find(|a| a.name == ANCHOR_PROP) else {
            return Ok(());
        };
        Err(self.source.error(
            written.offset,
            format!("`{ANCHOR_PROP}` is generated from the tag's `class`, `class:` and `style`: remove `{ANCHOR_PROP}={{..}}` and style the component with those"),
        ))
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
/// message: `declared` is a `.wisp` component, and a generated component
/// has no anchor element to make from the key (no `anchor_element`, no
/// `ANCHOR_TAG`), whatever props it declares.
pub(super) fn anchor_refusals(tag: &ComponentTag, declared: &Declared) -> Vec<(usize, String)> {
    let refused = tag
        .attributes
        .iter()
        .filter(|attribute| styles_anchor(attribute));
    refused
        .map(|attribute| {
            let message = format!(
                "`<{}>` takes no `{}`: it has no anchor element (a .wisp component has none; only a Rust component with an `{ANCHOR_PROP}` prop does); put it on an element inside {}",
                tag.name, attribute.name, declared.file
            );
            (attribute.offset, message)
        })
        .collect()
}
