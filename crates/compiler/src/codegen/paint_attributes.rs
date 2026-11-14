//! SVG presentation attributes on an `<svg>` or `<path>` (one-scene
//! S4a): a literal one is read by the cascade's own reading
//! (`vapors_css::cascade::declarations_of`), so a value the runtime
//! would refuse fails the build naming the file and line; a bound one
//! (`fill={..}`) is refused, as its run-time value could name a paint
//! the style analysis cannot see (svg-support 3.12).

use vapors_css::cascade::{PRESENTATION_ATTRIBUTES, declarations_of};

use crate::CompileError;
use crate::ast::{AttrValue, Attribute};
use crate::source::Source;

/// Refuses `attribute` on an element of `tag` when it is a presentation
/// attribute the runtime would refuse, or a bound one.
pub(super) fn refuse_paint(
    source: &Source,
    tag: &str,
    attribute: &Attribute,
) -> Result<(), CompileError> {
    let name = attribute.name.as_str();
    if !matches!(tag, "svg" | "path") || !PRESENTATION_ATTRIBUTES.contains(&name) {
        return Ok(());
    }
    let value = match &attribute.value {
        AttrValue::Literal(value) => value.as_str(),
        AttrValue::Expr(_) => return Err(source.error(attribute.offset, BOUND)),
    };
    declarations_of(tag, |asked| (asked == name).then_some(value))
        .map(|_| ())
        .map_err(|error| {
            source.error(
                attribute.offset,
                format!("{}; fix it in the template", error.message),
            )
        })
}

const BOUND: &str = "a bound SVG paint attribute is refused: the compiler must see every value it can take (docs/plans/svg-support.md 3.12); write the property under a class in the <style> section and toggle it with class:name={condition}, or set a custom property";
