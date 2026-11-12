//! What a player does to an app, as platform events: keys pressed and
//! released, and a click on a point; and where the racer's HUD puts
//! what is clicked.

use std::num::NonZeroU32;

use vapors_platform::{App, Event, Modifiers, Platform};
use vapors_racer::Racer;

/// The key `code` pressed or released.
pub fn key(app: &mut dyn App, code: &str, pressed: bool) {
    let key = if code == "Space" { " " } else { code };
    let event = Event::Key {
        key: key.into(),
        code: code.into(),
        pressed,
        repeat: false,
        modifiers: Modifiers::default(),
    };
    app.event(event, &Platform::detached());
}

/// The primary button pressed and released at `[x, y]`, the pointer
/// moved there first.
pub fn click(app: &mut dyn App, [x, y]: [f32; 2]) {
    let platform = Platform::detached();
    app.event(Event::PointerMoved { x, y }, &platform);
    for pressed in [true, false] {
        let button = Event::PrimaryButton {
            pressed,
            clicks: NonZeroU32::MIN,
            modifiers: Modifiers::default(),
        };
        app.event(button, &platform);
    }
}

/// The centre of the HUD element whose `attribute` is `value`, as drawn
/// by the last frame.
///
/// # Errors
/// No frame has been drawn, no element has the attribute, or it was not
/// laid out.
pub fn centre_of(racer: &Racer, attribute: &str, value: &str) -> Result<[f32; 2], String> {
    let document = racer.document();
    let document = document.borrow();
    let node = document
        .elements()
        .into_iter()
        .find(|&node| document.attribute(node, attribute) == Some(value))
        .ok_or_else(|| format!("the racer's HUD has no element with {attribute}=\"{value}\""))?;
    let painted = document
        .painted()
        .ok_or("the racer's HUD has not been drawn: run a frame before clicking it")?;
    let rect = painted
        .border_box(node)
        .ok_or_else(|| format!("the HUD's {attribute}=\"{value}\" was not laid out"))?;
    Ok([rect.x + rect.width / 2.0, rect.y + rect.height / 2.0])
}

/// Whether an element of the racer's HUD has focus, which takes Space.
#[must_use]
pub fn focused(racer: &Racer) -> bool {
    racer.document().borrow().active_element().is_some()
}
