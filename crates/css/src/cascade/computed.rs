//! The computed style of one element.

use std::collections::BTreeMap;

use vapors_scene::Color as Rgba;

use crate::ast::ComponentValue;
use crate::property::PropertyId;
use crate::value::Value;

/// Every longhand's computed value, the custom properties in scope, and
/// the two values other properties resolve against.
#[derive(Clone, Debug, PartialEq)]
pub struct ComputedStyle {
    pub(crate) values: Vec<Value>,
    pub(crate) custom: BTreeMap<String, Vec<ComponentValue>>,
    pub(crate) font_size: f64,
    pub(crate) root_font_size: f64,
    pub(crate) color: Rgba,
    pub(crate) positions: Vec<Option<crate::Position>>,
}

impl ComputedStyle {
    /// The computed value of a longhand.
    #[must_use]
    pub fn get(&self, id: PropertyId) -> &Value {
        &self.values[id.index()]
    }

    /// The computed value of a longhand by name; `None` for a name the
    /// table does not list as a longhand.
    #[must_use]
    pub fn get_by_name(&self, name: &str) -> Option<&Value> {
        PropertyId::from_name(name).map(|id| self.get(id))
    }

    /// A custom property's value (declared here or inherited), `var()`
    /// already substituted.
    #[must_use]
    pub fn custom(&self, name: &str) -> Option<&[ComponentValue]> {
        self.custom.get(name).map(Vec::as_slice)
    }

    /// Every custom property in scope.
    pub fn custom_properties(&self) -> impl Iterator<Item = (&str, &[ComponentValue])> {
        self.custom.iter().map(|(k, v)| (k.as_str(), v.as_slice()))
    }

    /// The computed `font-size` in px.
    #[must_use]
    pub fn font_size(&self) -> f64 {
        self.font_size
    }

    /// The computed `color`, which `currentcolor` elsewhere refers to.
    #[must_use]
    pub fn color(&self) -> Rgba {
        self.color
    }

    /// Where the winning declaration for `id` was written, if any was.
    #[must_use]
    pub fn declared_at(&self, id: PropertyId) -> Option<crate::Position> {
        self.positions[id.index()]
    }

    /// Every longhand with its computed value, in table order.
    pub fn iter(&self) -> impl Iterator<Item = (PropertyId, &Value)> {
        PropertyId::all().zip(self.values.iter())
    }
}
