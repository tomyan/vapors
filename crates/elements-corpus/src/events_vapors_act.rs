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

/// What the player holds between steps: the finger's press, the run of
/// taps the last steps were, and the clipboard's text, once a
/// `clipboard` step has set it.
#[derive(Default)]
pub(super) struct Held {
    press: Press,
    taps: Option<([f32; 2], u8)>,
    clipboard: Option<String>,
}

/// A tap at `point`: the touch's down and up, then the tap the
/// platform's gestures send for a still, quick touch, and its double tap
/// when it is the second of a run. Taps on consecutive steps at one
/// point are a run, counting 1, 2, 3 then again, as the platform's
/// `Gestures` counts them (the player has no clock: a script's steps
/// are as quick as Chromium's recording of them).
fn tap(doc: &Rc<RefCell<Document>>, point: [f32; 2], taps: &mut Option<([f32; 2], u8)>) {
    // the same script coordinates, parsed twice, not computed
    #[allow(clippy::float_cmp)]
    let count = match *taps {
        Some((last, count)) if last == point => count % 3 + 1,
        _ => 1,
    };
    *taps = Some((point, count));
    sent(doc, |doc| Document::touch_pressed(doc, 0, point));
    sent(doc, |doc| {
        Document::touch_released(doc, 0, point);
        Document::touch_tapped(doc, point);
        if count == 2 {
            Document::touch_double_tapped(doc, point);
        }
    });
}

#[allow(clippy::too_many_lines)] // one arm per script action
pub(super) fn act(
    doc: &Rc<RefCell<Document>>,
    action: &Action,
    held: &mut Held,
) -> Result<(), String> {
    if !matches!(action, Action::Tap(..)) {
        held.taps = None;
    }
    let press = &mut held.press;
    #[allow(clippy::cast_possible_truncation)] // CSS px within a viewport
    let at = |x: f64, y: f64| [x as f32, y as f32];
    match *action {
        Action::Click(x, y) => clicks(doc, at(x, y), 1),
        Action::Clicks(x, y, count) => clicks(doc, at(x, y), count),
        Action::AltClick(x, y) => {
            let alt = vapors_elements::Modifiers {
                alt: true,
                ..vapors_elements::Modifiers::default()
            };
            sent(doc, |doc| {
                Document::pointer_pressed_with(doc, at(x, y), std::num::NonZeroU32::MIN, alt);
            });
            sent(doc, |doc| {
                Document::pointer_released_with(doc, at(x, y), alt);
            });
        }
        Action::Key(ref combination) => key(doc, combination, held.clipboard.as_deref())?,
        Action::Clipboard(ref text) => held.clipboard = Some(text.clone()),
        // each character as the platform sends it: the key down, its
        // text, the key up
        Action::Type(ref text) => {
            for c in text.chars() {
                let key = c.to_string();
                let none = vapors_elements::Modifiers::default();
                sent(doc, |doc| {
                    Document::key_pressed(doc, &key, none);
                    Document::character_typed(doc, &key);
                });
                sent(doc, |doc| Document::key_released(doc, &key, none));
            }
        }
        Action::Tap(x, y) => tap(doc, at(x, y), &mut held.taps),
        Action::Move(x, y) => Document::pointer_moved(doc, at(x, y)),
        Action::Press(x, y) => {
            Document::pointer_pressed_clicks(doc, at(x, y), std::num::NonZeroU32::MIN);
        }
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
    Document::settle_selection(doc);
    Ok(())
}

/// A key down and up; Control+V, as the platform sends it, is the key
/// down then a paste of the clipboard's text in place of its character.
fn key(
    doc: &Rc<RefCell<Document>>,
    combination: &str,
    clipboard: Option<&str>,
) -> Result<(), String> {
    let (key, modifiers) = key_of(combination);
    sent(doc, |doc| {
        Document::key_pressed(doc, &key, modifiers);
    });
    if combination == "Control+v" {
        let text =
            clipboard.ok_or("Control+v before any `clipboard` step: set the clipboard first")?;
        // a paste no text field takes is the app's, which the player has not
        let _taken = Document::text_pasted(doc, text);
        Document::settle_selection(doc);
    }
    sent(doc, |doc| Document::key_released(doc, &key, modifiers));
    Ok(())
}

/// `count` presses and releases at `at`, each press counted one more,
/// as the app's `Clicks` counts presses close in time and place.
fn clicks(doc: &Rc<RefCell<Document>>, at: [f32; 2], count: u32) {
    for nth in (1..=count).filter_map(std::num::NonZeroU32::new) {
        // the press and its count, as one mousePressed is pointerdown
        // then mousedown in Chromium
        sent(doc, |doc| {
            Document::pointer_pressed_clicks(doc, at, nth);
        });
        sent(doc, |doc| Document::pointer_released(doc, at));
    }
}

/// Sends one platform event, then settles the selection, as the
/// Chromium player waits a task after each input call and Chromium
/// fires the `selectionchange` it queued then (E6a).
fn sent(doc: &Rc<RefCell<Document>>, event: impl FnOnce(&Rc<RefCell<Document>>)) {
    event(doc);
    Document::settle_selection(doc);
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
