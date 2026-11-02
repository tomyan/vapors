//! A text field the app has focused, for the platform's text input: the
//! web page's hidden textarea mirrors it, so a phone's keyboard and an
//! IME edit it, and the system IME places its candidate window at it.

/// The focused text field as the app holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct TextField {
    /// Which field it is, as the app tells its fields apart (an element
    /// node's id): a composition stays with the field it began in.
    pub id: u64,
    /// Its text.
    pub text: String,
    /// Its selection, start then end, in UTF-16 units.
    pub selection: (u32, u32),
    /// The caret's rect `[x, y, width, height]` in CSS px from the
    /// window's top left.
    pub area: [f32; 4],
}
