//! A number or text field's box: its text, which is the box's run, and
//! the focus marks paint draws over it (3-E3c, E6a).

use vapors_dom::NodeId;

use super::boxes::{LayoutBox, Source};

/// Fills in `field`'s focus marks when `node` is a number or text
/// field, and answers its text; `None` for another element.
pub(super) fn fill(source: &Source<'_>, node: NodeId, field: &mut LayoutBox) -> Option<String> {
    let element = source.arena.get(node, "build").element()?;
    let text_field = crate::text_field::is_text_input(element);
    let text = if text_field {
        crate::text_field::text_of(element)
    } else if element.tag == "input" && crate::check::is_type(element, "number") {
        crate::number::Field::of(&element.attributes, &element.properties).shown
    } else {
        return None;
    };
    let focused = |at: NodeId| at == node;
    field.number_focus = source
        .number_focus
        .filter(|(at, _)| focused(*at) && !text_field)
        .map(|(_, focus)| focus);
    field.text_field = text_field;
    field.text_marks = source
        .text_marks
        .as_ref()
        .filter(|(at, _)| focused(*at) && text_field)
        .map(|(_, marks)| marks.clone());
    Some(text)
}
