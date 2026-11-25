//! A number or text field's box: its text, which is the box's run, and
//! the focus marks paint draws over it (3-E3c, E6a), which the run
//! holds, so they are painted with the text, in the field's scroll
//! frame when it scrolls (entry 44).

use vapors_dom::NodeId;

use super::boxes::{LayoutBox, Source};

/// Fills in `field`'s focus marks when `node` is a number or text
/// field, and answers its text; `None` for another element. The marks
/// move to its run ([`mark_run`]).
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
    let text = crate::text_field::editor_text(element);
    if text.is_empty() {
        field.placeholder = crate::placeholder::text_of(element);
    }
    Some(text)
}

/// When box `field`, the box of `node`, is a text or number field,
/// gives its run the field's marks and names the field on it.
///
/// # Panics
/// When the field has no run: a field's text is its run, a blank one a
/// no-break space (`boxes.rs`).
pub(super) fn mark_run(boxes: &mut [LayoutBox], field: usize, node: NodeId) {
    if !boxes[field].text_field {
        return;
    }
    let run = *boxes[field].children.first().unwrap_or_else(|| {
        panic!("field {node:?} has no run: its text, a blank one a no-break space, is its run (boxes.rs `add_element`)")
    });
    boxes[run].field_text = Some(node);
    boxes[run].text_marks = boxes[field].text_marks.take();
}
