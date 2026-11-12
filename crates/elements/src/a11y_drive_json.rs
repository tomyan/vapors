//! A [`Command`] as JSON text, as stdio, the socket and the page call
//! carry it (R1): text that is no command is a [`Reply::Refused`] with
//! [`AxError::BadCommand`] naming the parse error, never a panic.

use std::cell::RefCell;
use std::rc::Rc;

use crate::a11y_drive::{Command, Reply};
use crate::a11y_error::AxError;
use crate::document::Document;

impl Document {
    /// Answers the command `text` holds, as [`Document::drive`]; text
    /// that is no command is refused as [`AxError::BadCommand`].
    pub fn drive_json(document: &Rc<RefCell<Document>>, text: &str) -> Reply {
        match serde_json::from_str::<Command>(text) {
            Ok(command) => Self::drive(document, &command),
            Err(error) => Reply::refused(AxError::BadCommand {
                error: error.to_string(),
            }),
        }
    }

    /// [`Document::drive_json`], the reply as JSON text; a reply that
    /// does not serialise is refused as [`AxError::BadReply`].
    pub fn drive_json_text(document: &Rc<RefCell<Document>>, text: &str) -> String {
        let reply = Self::drive_json(document, text);
        serde_json::to_string(&reply).unwrap_or_else(|error| bad_reply(&error))
    }
}

/// [`AxError::BadReply`] refused, written by hand: the serialiser just
/// failed, and a JSON value's text cannot.
fn bad_reply(error: &serde_json::Error) -> String {
    let problem = AxError::BadReply {
        error: error.to_string(),
    };
    serde_json::json!({
        "reply": "refused",
        "problem": "bad_reply",
        "error": error.to_string(),
        "message": problem.to_string(),
    })
    .to_string()
}
