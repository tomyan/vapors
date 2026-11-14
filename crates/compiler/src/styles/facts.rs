//! What one template element fixes, read through the one function
//! that builds its emitted key (`codegen::key_parts`), with its
//! refusals; plus its literal `style` parsed and the names of its
//! expression attributes: the `KeyFacts` the analysis judges rules
//! against.

use vapors_css::analysis::KeyFacts;
use vapors_css::ast::Declaration;

use crate::CompileError;
use crate::ast::{Attribute, Element};
use crate::codegen::attribute_kind::{AttributeKind, kind};
use crate::codegen::key_parts::key_parts;
use crate::codegen::style_key::Keyed;
use crate::source::Source;

/// The owned form of a `KeyFacts`, plus the literal `style` parsed
/// and the expression attributes, which the key does not fix.
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
    /// Every other literal attribute, as the key fixes them.
    pub literal_attributes: Vec<(String, String)>,
    /// Every `name={expr}` attribute's name, a run-time input.
    pub expression_attributes: Vec<String>,
    /// The literal `style`'s declarations.
    pub inline: Vec<Declaration>,
}

impl ElementFacts {
    /// The facts of `element`, refusing what its key would refuse
    /// (`key_parts`) and a literal `style` that does not parse.
    pub(super) fn of(
        element: &Element,
        scope: Option<&str>,
        source: &Source,
    ) -> Result<Self, CompileError> {
        let parts = key_parts(source, &Keyed::element(element))?;
        let inline = match attribute_named_style(element) {
            Some(attribute) => inline_declarations(attribute, source)?,
            None => Vec::new(),
        };
        Ok(Self {
            tag: element.tag.clone(),
            scope: scope.map(str::to_owned),
            classes: parts.classes.iter().map(|c| (*c).to_owned()).collect(),
            toggles: parts.toggles.iter().map(|t| (*t).to_owned()).collect(),
            style: parts.style.map(str::to_owned),
            literal_attributes: (parts.attributes.iter())
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
            expression_attributes: expression_names(element),
            inline,
        })
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

/// The names set by an expression, in template order.
fn expression_names(element: &Element) -> Vec<String> {
    (element.attributes.iter())
        .filter(|attribute| matches!(kind(attribute), AttributeKind::Expression(_)))
        .map(|attribute| attribute.name.clone())
        .collect()
}

fn attribute_named_style(element: &Element) -> Option<&Attribute> {
    (element.attributes.iter()).find(|attribute| kind(attribute) == AttributeKind::Style)
}

/// The literal `style` parsed as the runtime parses it at the first
/// update, refused at build instead, positioned at the attribute.
fn inline_declarations(
    attribute: &Attribute,
    source: &Source,
) -> Result<Vec<Declaration>, CompileError> {
    let text = attribute.value.literal().unwrap_or_default();
    vapors_css::parse_inline(text).map_err(|error| {
        source.error(
            attribute.offset,
            format!(
                "in the style attribute, {}: {}",
                error.position, error.message
            ),
        )
    })
}
