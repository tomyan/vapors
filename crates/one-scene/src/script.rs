//! Driving the example by its schedule, as a person would: before a
//! frame, a click at the centre of the light button due, and the
//! pointer back to where it rests: over the middle of the scene in the
//! schedule's pointer window, on the HUD panel otherwise (`confirm.rs`
//! checks after it). `vapors-perf`'s judged run and this crate's tests
//! drive it the same way.

use std::num::NonZeroU32;

use vapors_platform::{App, Event, Modifiers, Platform};

use crate::OneScene;
use crate::schedule::{click_at, pointing, time_of};

/// Where the pointer rests outside its window: inside the HUD panel
/// (16 px from the top left), over no mesh.
const ON_THE_HUD: [f32; 2] = [24.0, 24.0];

/// Makes the changes due before `frame`: the click on a light, if one is
/// due, and the pointer moved to where it rests when a click moved it
/// or its window opened or closed.
///
/// # Errors
/// The light's button, or the scene's surface, was not laid out by the
/// last frame.
pub fn drive(scene: &mut OneScene, frame: usize) -> Result<(), String> {
    let clicked = click_at(frame);
    if let Some(light) = clicked {
        let point = button_centre(scene, light + 1)?;
        click(scene, point);
    }
    let now = pointing(time_of(frame));
    let turned = frame > 0 && now != pointing(time_of(frame - 1));
    if clicked.is_some() || turned {
        let rest = if now {
            surface_centre(scene)?
        } else {
            ON_THE_HUD
        };
        move_to(scene, rest);
    }
    Ok(())
}

fn move_to(scene: &mut OneScene, [x, y]: [f32; 2]) {
    scene.event(Event::PointerMoved { x, y }, &Platform::detached());
}

/// The middle of the scene's surface, as the last frame drew it: the
/// camera looks at the group's centre there, so the pointer is over a
/// cube of the group whichever way it has turned.
fn surface_centre(scene: &OneScene) -> Result<[f32; 2], String> {
    let document = scene.document().borrow();
    let node = (document.elements().into_iter())
        .find(|&node| document.tag(node) == Some("gpu-surface"))
        .ok_or("the page has no gpu-surface")?;
    let painted = document
        .painted()
        .ok_or("no frame has been drawn: draw one before pointing")?;
    let rect = painted
        .border_box(node)
        .ok_or("the scene's surface was not laid out")?;
    let centre = [rect.x + rect.width / 2.0, rect.y + rect.height / 2.0];
    if painted.hit(centre) != Some(node) {
        return Err(format!(
            "the middle of the scene's surface, {centre:?}, is covered (the HUD, at this window size): drive the scene in a window at least 640x400"
        ));
    }
    Ok(centre)
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
