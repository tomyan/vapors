//! `style:<property>={expr}` (decision 2026-11-20-style-bindings): the
//! properties a template may bind, the typed value each takes, and the
//! declaration a bound value makes. Everything is an enum, so the
//! compiler's emitted calls are checked by rustc and a tree takes no
//! property by name.

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
    /// An opaque colour.
    #[must_use]
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

/// A bound value: a px length or a colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoundValue {
    /// A length in px, for a length property.
    Length(f32),
    /// A colour, for a colour property.
    Color(Rgba),
}

macro_rules! bindable {
    ($($variant:ident => $css:literal, $kind:ident;)*) => {
        /// A property a template may bind; [`BoundProperty::css_name`]
        /// is the name the author writes after `style:`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

            fn takes_length(self) -> bool {
                match self { $(Self::$variant => stringify!($kind) == "Length"),* }
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

    /// The declaration `value` makes for this property.
    ///
    /// # Errors
    /// A value of the wrong kind, or outside the domain (NaN, infinite,
    /// or negative where the property takes no negative), naming the fix.
    pub fn declaration(self, value: BoundValue) -> Result<String, String> {
        let name = self.css_name();
        match value {
            BoundValue::Length(px) if self.takes_length() => {
                if !px.is_finite() || (px < 0.0 && self.non_negative()) {
                    let domain = if self.non_negative() {
                        "0 or more"
                    } else {
                        "finite"
                    };
                    return Err(format!(
                        "`style:{name}` got {px}px, outside its domain ({domain}); bind a value computed in that range: it is never clamped"
                    ));
                }
                Ok(format!("{name}: {px}px"))
            }
            BoundValue::Color(c) if !self.takes_length() => Ok(format!(
                "{name}: #{:02x}{:02x}{:02x}{:02x}",
                c.r, c.g, c.b, c.a
            )),
            _ => Err(format!(
                "`style:{name}` takes {}: bind an expression of that type",
                if self.takes_length() {
                    "a px f32"
                } else {
                    "an Rgba"
                }
            )),
        }
    }
}

impl StyleKey {
    /// The declaration for binding `property` to `value` on an element
    /// of this key.
    ///
    /// # Errors
    /// A property the key does not list (only a template's
    /// `style:` binding lists one), or a value `declaration` refuses.
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
}
