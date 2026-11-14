//! Driving the example by its schedule, as a person would: before a
//! frame, a click at the centre of the light button due (`confirm.rs`
//! checks after it). `vapors-perf`'s judged run and this crate's tests
//! drive it the same way.

use std::num::NonZeroU32;

use vapors_platform::{App, Event, Modifiers, Platform};

use crate::OneScene;
use crate::schedule::click_at;

/// Makes the changes due before `frame`: the click on a light, if one is
/// due.
///
/// # Errors
/// The light's button was not laid out by the last frame.
pub fn drive(scene: &mut OneScene, frame: usize) -> Result<(), String> {
    let Some(light) = click_at(frame) else {
        return Ok(());
    };
    let point = button_centre(scene, light + 1)?;
    click(scene, point);
    Ok(())
}

/// The primary button pressed and released at `[x, y]`, the pointer
/// moved there first, as a mouse sends them.
fn click(scene: &mut OneScene, [x, y]: [f32; 2]) {
    let platform = Platform::detached();
    scene.event(Event::PointerMoved { x, y }, &platform);
    for pressed in [true, false] {
        let (clicks, modifiers) = (NonZeroU32::MIN, Modifiers::default());
        let button = Event::PrimaryButton {
            pressed,
            clicks,
            modifiers,
        };
        scene.event(button, &platform);
    }
}

/// The centre of the button for light `light`, as the last frame drew it.
fn button_centre(scene: &OneScene, light: usize) -> Result<[f32; 2], String> {
    let document = scene.document().borrow();
    let value = light.to_string();
    let node = (document.elements().into_iter())
        .find(|&node| document.attribute(node, "data-light") == Some(value.as_str()))
        .ok_or_else(|| format!("the HUD has no button for light {light}"))?;
    let painted = document
        .painted()
        .ok_or("no frame has been drawn: draw one before clicking")?;
    let rect = painted
        .border_box(node)
        .ok_or_else(|| format!("light {light}'s button was not laid out"))?;
    Ok([rect.x + rect.width / 2.0, rect.y + rect.height / 2.0])
}
