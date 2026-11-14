//! Driving the example by its schedule, as a person would: before a
//! frame, a click at the centre of the light button due; after it, a
//! check that every scheduled class reached its mesh. `vapors-perf`'s
//! judged run and this crate's tests drive it the same way.

use std::num::NonZeroU32;

use vapors_platform::{App, Event, Modifiers, Platform};
use vapors_scene3d::{NodeKind, Scene};

use crate::OneScene;
use crate::schedule::{CUBES, click_at, expected};

/// Makes the changes due before `frame`: the click on a light, if one is
/// due.
///
/// # Errors
/// The light's button was not laid out by the last frame.
pub fn drive(scene: &mut OneScene, frame: usize) -> Result<(), String> {
    let Some(light) = click_at(frame) else {
        return Ok(());
    };
    let [x, y] = button_centre(scene, light + 1)?;
    let platform = Platform::detached();
    scene.event(Event::PointerMoved { x, y }, &platform);
    for pressed in [true, false] {
        let modifiers = Modifiers::default();
        let clicks = NonZeroU32::MIN;
        scene.event(
            Event::PrimaryButton {
                pressed,
                clicks,
                modifiers,
            },
            &platform,
        );
    }
    Ok(())
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

/// Checks, after `frame`, that each cube's anchor has the classes the
/// schedule says, the stage its night class, and each mesh node the
/// colour and visibility its anchor computed.
///
/// # Errors
/// The first difference, naming the frame, the cube and what differs.
pub fn confirm(scene: &OneScene, frame: usize) -> Result<(), String> {
    let want = expected(frame);
    let document = scene.document().borrow();
    let anchors: Vec<_> = (document.elements().into_iter())
        .filter(|&node| document.tag(node) == Some("mesh"))
        .skip(1)
        .collect();
    let nodes = cube_nodes(&scene.scene().borrow())?;
    for (cube, (&anchor, node)) in anchors.iter().zip(&nodes).enumerate().take(CUBES) {
        let classes = document.attribute(anchor, "class").unwrap_or_default();
        let has = |class: &str| classes.split_whitespace().any(|c| c == class);
        let (lit, gone) = (want.lit[cube], cube == 2 && want.gone);
        if has("lit") != lit || has("gone") != gone {
            return Err(format!(
                "frame {frame}: cube {} has classes \"{classes}\"; the schedule wants lit {lit}, gone {gone}",
                cube + 1
            ));
        }
        let used = document
            .scene_used(anchor)
            .ok_or_else(|| format!("frame {frame}: cube {} has no used style", cube + 1))?;
        let drawn = (node.0, node.1);
        let colour = used.colour;
        let computed = ([colour.r, colour.g, colour.b], used.visible);
        if drawn != computed {
            return Err(format!(
                "frame {frame}: cube {} is drawn {drawn:?}; its anchor computed {computed:?}",
                cube + 1
            ));
        }
    }
    Ok(())
}

/// Each cube's drawn colour channels and visibility, in the group's order.
fn cube_nodes(scene: &Scene) -> Result<Vec<([f32; 3], bool)>, String> {
    let group = (scene
        .children(scene.root())
        .map_err(|e| e.to_string())?
        .iter())
    .copied()
    .find(|&node| matches!(scene.kind(node), Ok(NodeKind::Group)))
    .ok_or("the scene has no group of cubes")?;
    let cubes = scene.children(group).map_err(|e| e.to_string())?;
    cubes
        .iter()
        .map(|&node| {
            let NodeKind::Mesh { colour, .. } = scene.kind(node).map_err(|e| e.to_string())? else {
                return Err(format!("{node} in the group is not a mesh"));
            };
            let [r, g, b, _] = colour.rgba();
            Ok(([r, g, b], scene.visible(node).map_err(|e| e.to_string())?))
        })
        .collect()
}
