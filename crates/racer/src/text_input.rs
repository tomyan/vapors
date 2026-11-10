//! Forward the platform text field to the same compiled numeric controls.
use std::{cell::RefCell, rc::Rc};
use vapors_elements::{Document, TextEdit};
use vapors_platform::{Event, Platform, TextField};
pub(crate) fn edit(document: &Rc<RefCell<Document>>, event: &Event) -> Result<(), String> {
    match event {
        Event::TextEdited {
            before,
            text,
            selection,
            composing,
        } => Document::text_edited(
            document,
            &TextEdit {
                before: before.clone(),
                text: text.clone(),
                selection: (selection.0 as usize, selection.1 as usize),
                composing: composing.map(|(a, b)| (a as usize, b as usize)),
            },
        ),
        Event::PastedText(text) if !Document::text_pasted(document, text) => {
            Err("racer paste refused: focus a text field before pasting".into())
        }
        _ => Ok(()),
    }
}
pub(crate) fn mirror(document: &Rc<RefCell<Document>>, platform: &Platform) {
    let field = document.borrow_mut().text_input().map(|input| TextField {
        id: input.node.0,
        text: input.text,
        selection: (
            u32::try_from(input.selection.0).expect("short input"),
            u32::try_from(input.selection.1).expect("short input"),
        ),
        area: input.area,
    });
    platform.set_text_field(field);
}
