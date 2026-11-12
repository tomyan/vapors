//! A number or text field's box: its text, which is the box's run, and
//! the focus marks paint draws over it (3-E3c, E6a).

use vapors_dom::NodeId;

use super::boxes::{LayoutBox, Source};

/// Fills in `field`'s focus marks when `node` is a number or text
/// field, and answers its text; `None` for another element.
pub(super) fn fill(source: &Source<'_>, node: NodeId, field: &mut LayoutBox) -> Option<String> {
    let element = source
        .arena
        .get(node, "build")
        .element()
        .filter(|element| crate::text_field::is_edited(element))?;
    field.text_field = true;
    field.text_marks = source
        .text_marks
        .as_ref()
        .filter(|(at, _)| *at == node)
        .map(|(_, marks)| marks.clone());
    Some(crate::text_field::editor_text(element))
}
