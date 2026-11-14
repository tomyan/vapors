//! The check after each frame of a driven run: every mesh is drawn in
//! the colour and visibility the schedule says, whatever path the change
//! took (a click's class, the time signal's class, the night token).

use vapors_scene3d::{NodeId, NodeKind, Scene};

use crate::OneScene;
use crate::schedule::{CUBES, expected};

/// Checks, after `frame`, that the ground and each cube are drawn as
/// the schedule says.
///
/// # Errors
/// The scene is not one ground and [`CUBES`] cubes, or the first mesh
/// drawn otherwise than scheduled, naming the frame and both.
pub fn confirm(scene: &OneScene, frame: usize) -> Result<(), String> {
    let scene = scene.scene().borrow();
    let (ground, cubes) = meshes(&scene)?;
    let want = expected(frame);
    check(
        frame,
        "the ground",
        drawn(&scene, ground)?,
        (want.ground(), true),
    )?;
    for (cube, &node) in cubes.iter().enumerate() {
        check(
            frame,
            &format!("cube {}", cube + 1),
            drawn(&scene, node)?,
            want.cube(cube),
        )?;
    }
    Ok(())
}

fn check(
    frame: usize,
    what: &str,
    drawn: ([u8; 3], bool),
    want: ([u8; 3], bool),
) -> Result<(), String> {
    if drawn == want {
        return Ok(());
    }
    Err(format!(
        "frame {frame}: {what} is drawn {drawn:?} (colour, shown); the schedule wants {want:?}"
    ))
}

/// The ground (the root's one mesh) and the cubes (the group's meshes).
fn meshes(scene: &Scene) -> Result<(NodeId, Vec<NodeId>), String> {
    let top = children(scene, scene.root())?;
    let ground = only(scene, &top, "mesh", |kind| {
        matches!(kind, NodeKind::Mesh { .. })
    })?;
    let group = only(scene, &top, "group", |kind| matches!(kind, NodeKind::Group))?;
    let cubes = children(scene, group)?;
    if cubes.len() != CUBES {
        return Err(format!(
            "the group holds {} nodes; the page builds {CUBES} cubes",
            cubes.len()
        ));
    }
    Ok((ground, cubes))
}

/// The one node of `nodes` of the kind `is` picks, named `what`.
fn only(
    scene: &Scene,
    nodes: &[NodeId],
    what: &str,
    is: fn(&NodeKind) -> bool,
) -> Result<NodeId, String> {
    let found: Vec<NodeId> = (nodes.iter().copied())
        .filter(|&node| scene.kind(node).is_ok_and(is))
        .collect();
    match found.as_slice() {
        [one] => Ok(*one),
        _ => Err(format!(
            "the scene's root holds {} of kind {what}; the page builds one",
            found.len()
        )),
    }
}

fn children(scene: &Scene, node: NodeId) -> Result<Vec<NodeId>, String> {
    scene
        .children(node)
        .map(<[NodeId]>::to_vec)
        .map_err(|e| e.to_string())
}

/// `node`'s drawn colour as sRGB bytes, and whether it is shown.
fn drawn(scene: &Scene, node: NodeId) -> Result<([u8; 3], bool), String> {
    let NodeKind::Mesh { colour, .. } = scene.kind(node).map_err(|e| e.to_string())? else {
        return Err(format!("{node} is not a mesh"));
    };
    let [r, g, b, _] = colour.rgba().map(byte);
    Ok(([r, g, b], scene.visible(node).map_err(|e| e.to_string())?))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // a channel from 0 to 1
fn byte(channel: f32) -> u8 {
    (channel * 255.0).round() as u8
}
