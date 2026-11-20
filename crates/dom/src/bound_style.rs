//! `style:<property>={expr}` (decision 2026-11-20-style-bindings): the
//! bindable properties, the typed value each takes and the declaration
//! a value makes. All enums, so rustc checks the compiler's calls.

use crate::StyleKey;

/// An 8-bit sRGB colour with alpha, what a colour property takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgba {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha, 255 opaque.
    pub a: u8,
}

impl Rgba {
    /// Opaque.
    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

/// A bound value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoundValue {
    /// A px length.
    Length(f32),
    /// A colour, for a colour property.
    Color(Rgba),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Takes {
    Length,
    Color,
}

macro_rules! bindable {
    ($($variant:ident => $css:literal, $kind:ident;)*) => {
        /// A property a template may bind; [`BoundProperty::css_name`]
        /// is the name the author writes after `style:`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        #[allow(missing_docs)] // each is its `css_name`
        pub enum BoundProperty { $($variant),* }

        impl BoundProperty {
            /// Every bindable property.
            pub const ALL: &'static [BoundProperty] = &[$(Self::$variant),*];

            /// The CSS property.
            #[must_use]
            pub fn css_name(self) -> &'static str {
                match self { $(Self::$variant => $css),* }
            }

            fn takes(self) -> Takes {
                match self { $(Self::$variant => Takes::$kind),* }
            }
        }
    };
}

bindable! {
    Width => "width", Length; Height => "height", Length;
    Top => "top", Length; Left => "left", Length;
    MarginTop => "margin-top", Length; MarginRight => "margin-right", Length;
    MarginBottom => "margin-bottom", Length; MarginLeft => "margin-left", Length;
    PaddingTop => "padding-top", Length; PaddingRight => "padding-right", Length;
    PaddingBottom => "padding-bottom", Length; PaddingLeft => "padding-left", Length;
    BackgroundColor => "background-color", Color; Color => "color", Color;
}

impl BoundProperty {
    /// Whether a length below 0 is outside the property's domain.
    fn non_negative(self) -> bool {
        use BoundProperty::{Height, PaddingBottom, PaddingLeft, PaddingRight, PaddingTop, Width};
        matches!(
            self,
            Width | Height | PaddingTop | PaddingRight | PaddingBottom | PaddingLeft
        )
    }

    fn length(self, px: f32) -> Result<String, String> {
        let name = self.css_name();
        if px.is_finite() && !(px < 0.0 && self.non_negative()) {
            // `+ 0.0`: -0 prints as 0
            return Ok(format!("{name}: {}px", px + 0.0));
        }
        let domain = if self.non_negative() {
            "0 or more"
        } else {
            "finite"
        };
        Err(format!(
            "`style:{name}` got {px} (px), outside its domain ({domain}); bind a value computed in that range: it is never clamped"
        ))
    }

    /// The declaration `value` makes for this property.
    ///
    /// # Errors
    /// A value of the wrong kind or outside the domain, naming the fix.
    pub fn declaration(self, value: BoundValue) -> Result<String, String> {
        let name = self.css_name();
        match (self.takes(), value) {
            (Takes::Length, BoundValue::Length(px)) => self.length(px),
            (Takes::Color, BoundValue::Color(c)) => Ok(format!(
                "{name}: #{:02x}{:02x}{:02x}{:02x}",
                c.r, c.g, c.b, c.a
            )),
            (takes, _) => Err(format!(
                "`style:{name}` takes {}: bind an expression of that type",
                if takes == Takes::Length {
                    "a px f32"
                } else {
                    "an Rgba"
                }
            )),
        }
    }
}

/// The one refusal every tree gives for a binding on an unkeyed element.
#[must_use]
pub fn unkeyed_binding_refusal(tag: &str) -> String {
    format!(
        "<{tag}> was not created by create_keyed_element, so it has no `style:` binding; create it from its template's StyleKey"
    )
}

impl StyleKey {
    /// The declaration for binding `property` to `value` here.
    ///
    /// # Errors
    /// A property the key does not list, or a value `declaration` refuses.
    pub fn bound_declaration(
        &self,
        property: BoundProperty,
        value: BoundValue,
    ) -> Result<String, String> {
        if !self.bound.contains(&property) {
            return Err(format!(
                "{}: `style:{}` is not bound in this element's template; write style:{0}={{expr}} on the element in its .wisp: a property changes only through a template binding",
                self.site,
                property.css_name()
            ));
        }
        property
            .declaration(value)
            .map_err(|why| format!("{}: {why}", self.site))
    }

    pub(crate) fn repeated_binding(&self) -> Option<String> {
        let (_, property) = (self.bound.iter().enumerate())
            .find(|(index, property)| self.bound[..*index].contains(property))?;
        Some(format!(
            "lists `style:{}` twice; bind each property once",
            property.css_name()
        ))
    }
}
