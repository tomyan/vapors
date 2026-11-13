//! What one template element fixes, read from the AST as
//! `codegen/style_key.rs` reads its key, and its literal `style`
//! parsed: the `KeyFacts` the analysis judges rules against.

use vapors_css::analysis::KeyFacts;
use vapors_css::ast::Declaration;

use crate::CompileError;
use crate::ast::{AttrValue, Attribute, Element};
use crate::source::Source;

/// The owned form of a `KeyFacts`, plus the literal `style` parsed.
#[derive(Debug)]
pub struct ElementFacts {
    /// The tag.
    pub tag: String,
    /// The scope the element is tagged with, when the component has a
    /// scoped style.
    pub scope: Option<String>,
    /// The literal `class` names.
    pub classes: Vec<String>,
    /// The `class:` directive names.
    pub toggles: Vec<String>,
    /// The literal `style` text.
    pub style: Option<String>,
    /// Every other literal attribute.
    pub literal_attributes: Vec<(String, String)>,
    /// Every `name={expr}` attribute's name, a run-time input the key
    /// does not fix. Listeners (`on*`) and bindings (`bind:*`) set no
    /// attribute, as `codegen/element.rs` treats them.
    pub expression_attributes: Vec<String>,
    /// The literal `style`'s declarations.
    pub inline: Vec<Declaration>,
}

impl ElementFacts {
    pub(super) fn of(
        element: &Element,
        scope: Option<&str>,
        source: &Source,
    ) -> Result<Self, CompileError> {
        let mut facts = Self {
            tag: element.tag.clone(),
            scope: scope.map(str::to_owned),
            classes: Vec::new(),
            toggles: Vec::new(),
            style: None,
            literal_attributes: Vec::new(),
            expression_attributes: Vec::new(),
            inline: Vec::new(),
        };
        for attribute in element.attributes.iter() {
            facts.take(attribute, source)?;
        }
        Ok(facts)
    }

    fn take(&mut self, attribute: &Attribute, source: &Source) -> Result<(), CompileError> {
        let name = attribute.name.as_str();
        if let Some(toggle) = name.strip_prefix("class:") {
            self.toggles.push(toggle.to_owned());
            return Ok(());
        }
        let listener = name.strip_prefix("on").is_some_and(|rest| !rest.is_empty());
        if listener || name.starts_with("bind:") {
            return Ok(());
        }
        let AttrValue::Literal(value) = &attribute.value else {
            // `class={..}` and `style={..}` are refused by codegen
            self.expression_attributes.push(name.to_owned());
            return Ok(());
        };
        if name.eq_ignore_ascii_case("class") {
            self.classes = value.split_ascii_whitespace().map(str::to_owned).collect();
        } else if name.eq_ignore_ascii_case("style") {
            self.inline = inline_declarations(value, attribute, source)?;
            self.style = Some(value.clone());
        } else {
            self.literal_attributes
                .push((name.to_owned(), value.clone()));
        }
        Ok(())
    }

    /// Runs `with` on the facts as the analysis borrows them.
    pub fn with_key<R>(&self, with: impl FnOnce(&KeyFacts<'_>) -> R) -> R {
        let classes: Vec<&str> = self.classes.iter().map(String::as_str).collect();
        let toggles: Vec<&str> = self.toggles.iter().map(String::as_str).collect();
        let literal: Vec<(&str, &str)> = (self.literal_attributes.iter())
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        with(&KeyFacts {
            tag: &self.tag,
            scope_attribute: crate::SCOPE_ATTRIBUTE,
            scope: self.scope.as_deref(),
            classes: &classes,
            toggles: &toggles,
            style: self.style.as_deref(),
            literal_attributes: &literal,
        })
    }
}

/// The literal `style` parsed as the runtime parses it at the first
/// update, refused at build instead, positioned at the attribute.
fn inline_declarations(
    value: &str,
    attribute: &Attribute,
    source: &Source,
) -> Result<Vec<Declaration>, CompileError> {
    vapors_css::parse_inline(value).map_err(|error| {
        source.error(
            attribute.offset,
            format!(
                "in the style attribute, {}: {}",
                error.position, error.message
            ),
        )
    })
}
