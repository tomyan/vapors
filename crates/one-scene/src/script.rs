//! Driving the example by its schedule, as a person would: before a
//! frame, a click at the centre of the light button due, and the
//! pointer moved to where the schedule has it rest whenever it is not
//! there: over the middle of the scene in the pointer's window, on the
//! HUD panel otherwise (`confirm.rs` checks after it). `vapors-perf`'s
//! judged run and this crate's tests drive it the same way.

use std::num::NonZeroU32;

use vapors_dom::NodeId;
use vapors_elements::{Document, Painted};
use vapors_platform::{App, Event, Modifiers, Platform};

use crate::OneScene;
use crate::schedule::{click_at, pointing, time_of};

/// Where the pointer rests outside its window: inside the HUD panel
/// (16 px from the top left), over no mesh; checked each time it is used.
const ON_THE_HUD: [f32; 2] = [24.0, 24.0];

/// Where the script last left the pointer resting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Rest {
    /// Over the middle of the scene.
    OverTheScene,
    /// On the HUD panel.
    OnTheHud,
}

/// Makes the changes due before `frame`: the click on a light, if one is
/// due, and the pointer moved to where it rests at `frame` when a click
/// moved it or it rests elsewhere.
///
/// # Errors
/// The light's button, the scene's surface or the HUD was not laid out
/// by the last frame, or a rest point is covered.
pub fn drive(scene: &mut OneScene, frame: usize) -> Result<(), String> {
    if let Some(light) = click_at(frame) {
        let point = button_centre(scene, light + 1)?;
        click(scene, point);
        scene.rest = None;
    }
    let wanted = if pointing(time_of(frame)) {
        Rest::OverTheScene
    } else {
        Rest::OnTheHud
    };
    if scene.rest == Some(wanted) {
        return Ok(());
    }
    let point = match wanted {
        Rest::OverTheScene => surface_centre(scene)?,
        Rest::OnTheHud => on_the_hud(scene)?,
    };
    move_to(scene, point);
    scene.rest = Some(wanted);
    Ok(())
}

fn move_to(scene: &mut OneScene, [x, y]: [f32; 2]) {
    scene.event(Event::PointerMoved { x, y }, &Platform::detached());
}

/// The primary button pressed and released at `[x, y]`, the pointer
/// moved there first, as a mouse sends them.
fn click(scene: &mut OneScene, point: [f32; 2]) {
    move_to(scene, point);
    let platform = Platform::detached();
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

/// The middle of the scene's surface, as the last frame drew it: the
/// camera looks at the group's centre there, so the pointer is over a
/// cube of the group whichever way it has turned. Refused when anything
/// covers it.
fn surface_centre(scene: &OneScene) -> Result<[f32; 2], String> {
    let document = scene.document().borrow();
    let surface = find(&document, |node| document.tag(node) == Some("gpu-surface"))
        .ok_or("the page has no gpu-surface")?;
    let painted = painted(&document)?;
    let centre = centre(painted, surface, "the scene's surface")?;
    if painted.hit(centre) != Some(surface) {
        return Err(format!(
            "the middle of the scene's surface, {centre:?}, is covered (the HUD, at this window size): drive the scene in a window at least 640x400"
        ));
    }
    Ok(centre)
}

/// [`ON_THE_HUD`], refused unless the last frame drew the HUD panel
/// there.
fn on_the_hud(scene: &OneScene) -> Result<[f32; 2], String> {
    let document = scene.document().borrow();
    let hit = painted(&document)?.hit(ON_THE_HUD);
    if hit.is_some_and(|node| in_hud(&document, node)) {
        return Ok(ON_THE_HUD);
    }
    Err(format!(
        "{ON_THE_HUD:?} is not on the HUD panel ({hit:?} is there): move ON_THE_HUD in script.rs to a point the panel covers"
    ))
}

/// Whether `node` is the HUD panel or inside it.
fn in_hud(document: &Document, node: NodeId) -> bool {
    let is_hud = |node| {
        document
            .attribute(node, "class")
            .is_some_and(|class| class.split(' ').any(|c| c == "hud"))
    };
    std::iter::successors(Some(node), |&node| document.parent(node)).any(is_hud)
}

/// The centre of the button for light `light`, as the last frame drew it.
fn button_centre(scene: &OneScene, light: usize) -> Result<[f32; 2], String> {
    let document = scene.document().borrow();
    let value = light.to_string();
    let node = find(&document, |node| {
        document.attribute(node, "data-light") == Some(value.as_str())
    })
    .ok_or_else(|| format!("the HUD has no button for light {light}"))?;
    centre(
        painted(&document)?,
        node,
        &format!("light {light}'s button"),
    )
}

fn find(document: &Document, is: impl Fn(NodeId) -> bool) -> Option<NodeId> {
    document.elements().into_iter().find(|&node| is(node))
}

fn painted(document: &Document) -> Result<&Painted, String> {
    document
        .painted()
        .ok_or_else(|| "no frame has been drawn: draw one before driving".to_owned())
}

/// The centre of `node`'s border box, named `what` when it has none.
fn centre(painted: &Painted, node: NodeId, what: &str) -> Result<[f32; 2], String> {
    let rect = painted
        .border_box(node)
        .ok_or_else(|| format!("{what} was not laid out"))?;
    Ok([rect.x + rect.width / 2.0, rect.y + rect.height / 2.0])
}
