//! The vapors player's actions (`events_vapors`): each `.events` action
//! sent through the document's pointer, touch and key calls.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_elements::Document;

use crate::events_script::Action;

/// The platform's tap slop (`vapors_platform::TAP_SLOP`, which this
/// crate does not depend on; `tests/events_vapors.rs` holds the two
/// equal): a finger pans only once it has moved further than this from
/// where it went down, as the platform's `Gestures` sends pans.
pub const TAP_SLOP: f32 = 20.0;

/// Where the finger went down, while it is still inside the tap slop.
#[derive(Default)]
pub(super) struct Press(Option<[f32; 2]>);

pub(super) fn act(
    doc: &Rc<RefCell<Document>>,
    action: &Action,
    press: &mut Press,
) -> Result<(), String> {
    #[allow(clippy::cast_possible_truncation)] // CSS px within a viewport
    let at = |x: f64, y: f64| [x as f32, y as f32];
    match *action {
        Action::Click(x, y) => {
            Document::pointer_pressed(doc, at(x, y));
            Document::pointer_released(doc, at(x, y));
        }
        Action::Key(ref combination) => {
            let (key, modifiers) = key_of(combination);
            Document::key_pressed(doc, &key, modifiers);
            Document::key_released(doc, &key, modifiers);
        }
        Action::Move(x, y) => Document::pointer_moved(doc, at(x, y)),
        Action::Press(x, y) => Document::pointer_pressed(doc, at(x, y)),
        Action::Release(x, y) => Document::pointer_released(doc, at(x, y)),
        Action::TouchDown(x, y) => {
            press.0 = Some(at(x, y));
            Document::touch_pressed(doc, 0, at(x, y));
        }
        Action::TouchMove(x, y) => {
            Document::touch_moved(doc, 0, at(x, y));
            if press.left(at(x, y)) {
                // past the slop the platform pans, which the app hands to
                // `touch_panned`
                Document::touch_panned(doc);
            }
        }
        // inside the slop the platform would send a tap, which this
        // player does not model: refused, not approximated
        Action::TouchUp(..) if press.0.is_some() => {
            return Err(
                "a touch that never left the tap slop is a tap, which is not played on vapors yet"
                    .into(),
            );
        }
        Action::TouchUp(x, y) => Document::touch_released(doc, 0, at(x, y)),
        ref other => return Err(format!("{other:?} is not played on vapors yet")),
    }
    Ok(())
}

impl Press {
    /// Whether the finger, now at `at`, has left the tap slop (it stays
    /// out once it has).
    fn left(&mut self, at: [f32; 2]) -> bool {
        let Some(down) = self.0 else { return true };
        if (at[0] - down[0]).hypot(at[1] - down[1]) <= TAP_SLOP {
            return false;
        }
        self.0 = None;
        true
    }
}

/// A script's `key` (`Shift+Tab`, `Space`, `x`) as the DOM's key and the
/// modifiers held, as the Chromium player sends it.
fn key_of(combination: &str) -> (String, vapors_elements::Modifiers) {
    let parts: Vec<&str> = combination.split('+').collect();
    let (held, key) = parts.split_at(parts.len() - 1);
    let modifiers = vapors_elements::Modifiers {
        shift: held.contains(&"Shift"),
        control: held.contains(&"Control"),
        alt: held.contains(&"Alt"),
        meta: held.contains(&"Meta"),
    };
    let key = match key[0] {
        "Space" => " ".to_owned(),
        other => other.to_owned(),
    };
    (key, modifiers)
}
