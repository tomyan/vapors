//! Each template element's `vapors_dom::StyleKey`: a file-level
//! `static` named after the element's node (`KEY_DIV0` for `div0`)
//! holding what the template says about it (its site, scope, tag,
//! literal classes, `class:` directive names, literal `style` and every
//! other literal attribute), and the `create_keyed_element(&KEY_DIV0)`
//! that builds the element from it. The tree sets those attributes
//! from the key and refuses a change to them, so nothing else is
//! emitted for them. A component tag carrying `class`, `class:` or
//! `style` gets a key the same way, which it passes to the component
//! (`component_anchor`); its other attributes are props, not carried.

use super::emit::Emitter;
use super::key_parts::key_parts;
use crate::CompileError;
use crate::ast::{Attributes, ComponentTag, Element};

/// What a key is made from: an element, whose key's tag is its own, or
/// a component tag, whose key's tag is the `ANCHOR_TAG` its props type
/// declares, so an alias (`Group as Section`) still keys a `group`.
pub(crate) struct Keyed<'e> {
    /// The key's `tag` field, as Rust.
    pub(super) tag: String,
    /// `div` or `Mesh`, for messages.
    pub(super) name: &'e str,
    /// Where `class:` directives past the limit can go instead.
    pub(super) elsewhere: String,
    /// Whether the key carries the literal attributes (an element's;
    /// a component tag's are props).
    pub(super) carries_attributes: bool,
    /// An element, which the styles analysis sees, or a component tag,
    /// whose key's tag its props type decides.
    pub(super) kind: KeyedKind,
    pub(super) attributes: &'e Attributes,
    pub(super) offset: usize,
}

/// What a key is made from, for its winners.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyedKind {
    Element,
    ComponentTag,
}

impl<'e> Keyed<'e> {
    pub(crate) fn element(element: &'e Element) -> Self {
        Self {
            tag: format!("{:?}", element.tag),
            name: &element.tag,
            elsewhere: "a child element".to_owned(),
            carries_attributes: true,
            kind: KeyedKind::Element,
            attributes: &element.attributes,
            offset: element.offset,
        }
    }

    pub(super) fn component(tag: &'e ComponentTag) -> Self {
        Self {
            tag: format!("{}Props::ANCHOR_TAG", tag.name),
            name: &tag.name,
            elsewhere: format!("an element inside <{}>", tag.name),
            carries_attributes: false,
            kind: KeyedKind::ComponentTag,
            attributes: &tag.attributes,
            offset: tag.offset,
        }
    }
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
        let winners = self.winners_static(keyed, var)?;
        let key = self.style_key(keyed, &winners)?;
        self.keys.push(format!(
            "static {name}: vapors_elements::__wisp::dom::StyleKey = {key};"
        ));
        Ok(name)
    }

    /// `Winners::Compiled(&WINNERS_{VAR})` for an element, after adding
    /// that static; `Winners::ComponentTag` for a component tag, whose
    /// key's tag its props type decides, which the analysis does not
    /// see. Every generation analyses (`generate` with the crate's
    /// other sheets or none, whose rules the runtime then adds as
    /// foreign); only the emission that looks for codegen's own refusal
    /// after the analysis refused has none, and its output is dropped.
    fn winners_static(&mut self, keyed: &Keyed, var: &str) -> Result<String, CompileError> {
        if keyed.kind == KeyedKind::ComponentTag {
            return Ok("vapors_elements::__wisp::dom::Winners::ComponentTag".to_owned());
        }
        let Some(winners) = self.winners else {
            return Ok("vapors_elements::__wisp::dom::Winners::Unanalysed".to_owned());
        };
        let (line, column) = self.source.position(keyed.offset);
        let literal = winners.literal(line, column).ok_or_else(|| {
            self.source.error(
                keyed.offset,
                "the styles analysis holds no element here, so its winners cannot be emitted; \
                 a defect in vapors-compiler (styles::walk and codegen disagree on the elements): \
                 report it with this file",
            )
        })?;
        let literal = literal.map_err(|why| self.source.error(keyed.offset, why))?;
        let name = format!("WINNERS_{}", var.to_ascii_uppercase());
        self.keys.push(format!(
            "static {name}: vapors_elements::__wisp::dom::KeyWinners = {literal};"
        ));
        Ok(format!(
            "vapors_elements::__wisp::dom::Winners::Compiled(&{name})"
        ))
    }

    fn style_key(&self, keyed: &Keyed, winners: &str) -> Result<String, CompileError> {
        let (line, column) = self.source.position(keyed.offset);
        // the package, as `SCOPE` names it, so two crates' `src/card.wisp` differ
        let place = format!("{}:{line}:{column}", self.site_name);
        let site = format!("concat!(env!(\"CARGO_PKG_NAME\"), \"/\", {place:?})");
        let scope = if self.scoped { "Some(SCOPE)" } else { "None" };
        let parts = key_parts(self.source, keyed)?;
        let attributes: Vec<String> = (parts.attributes.iter())
            .map(|(name, value)| format!("({name:?}, {value:?})"))
            .collect();
        Ok(format!(
            "vapors_elements::__wisp::dom::StyleKey {{ site: {site}, scope: {scope}, tag: {}, classes: &[{}], toggles: &[{}], style: {}, attributes: &[{}], bound: &[{}], winners: {winners} }}",
            keyed.tag,
            quoted(&parts.classes),
            quoted(&parts.toggles),
            parts
                .style
                .map_or("None".to_owned(), |text| format!("Some({text:?})")),
            attributes.join(", "),
            super::style_binding::bound_literal(&parts.bound)
        ))
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
