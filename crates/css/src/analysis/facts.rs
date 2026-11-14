//! What a template key fixes about every element it makes: the
//! `KeyFacts` the classification judges rules against, and the shape
//! it promises.

/// What a template key fixes about every element it makes: the
/// `StyleKey` the compiler emits, read field by field, and nothing
/// else (the compiler builds both from one list of the element's
/// attributes). It has the shape `StyleKey::shape_refusal` accepts:
/// each class and directive one name, no name in both;
/// `literal_attributes` holds neither `class`, `style` nor the scope
/// attribute (compared ASCII-case-insensitively, as the key does) and
/// no name twice. [`classify`] checks that in debug builds. Every
/// attribute not listed here is a run-time input: `set_attribute` may
/// set it.
#[derive(Clone, Copy, Debug)]
pub struct KeyFacts<'a> {
    /// The element's tag.
    pub tag: &'a str,
    /// The attribute scoped sheets test (`data-wisp`).
    pub scope_attribute: &'a str,
    /// Its value on the element; `None` in a component without a style.
    pub scope: Option<&'a str>,
    /// The literal `class` names.
    pub classes: &'a [&'a str],
    /// The `class:` directive names, each on or off at run time.
    pub toggles: &'a [&'a str],
    /// The literal `style` text.
    pub style: Option<&'a str>,
    /// Every other literal attribute, by name as written, which the key
    /// fixes at run time as it fixes `class` and `style`.
    pub literal_attributes: &'a [(&'a str, &'a str)],
}

impl KeyFacts<'_> {
    /// The shape `StyleKey::shape_refusal` and the fields above promise,
    /// judged as `StyleKey::attribute_misplaced` judges it.
    pub(super) fn check_shape(&self) {
        debug_assert!(
            self.toggles.iter().all(|t| !self.classes.contains(t)),
            "a directive is also a literal class: {self:?}"
        );
        debug_assert!(
            self.literal_attributes.iter().all(|(name, _)| {
                !name.eq_ignore_ascii_case("class")
                    && !name.eq_ignore_ascii_case("style")
                    && !name.eq_ignore_ascii_case(self.scope_attribute)
            }),
            "class, style and the scope attribute are the key's own fields: {self:?}"
        );
        debug_assert!(
            (self.literal_attributes.iter().enumerate()).all(|(index, (name, _))| {
                !self.literal_attributes[..index]
                    .iter()
                    .any(|(earlier, _)| earlier.eq_ignore_ascii_case(name))
            }),
            "an attribute is listed twice: {self:?}"
        );
    }
}
