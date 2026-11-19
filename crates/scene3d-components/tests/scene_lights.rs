//! `<Light>` mounted through a `.wisp` page: one light node each, in
//! the scene's lights, moved by its transform, at most eight.

mod support;

use support::at;
use support::lights::{LightsRig, white};
use vapors_scene3d::{Ambient, Light, Vec3};

#[test]
fn the_page_mounts_one_light_node_per_tag_and_sets_the_ambient() {
    // Given / When
    let ambient = Ambient::new(white(), 0.25).unwrap();
    let rig = LightsRig::new(2, ambient);

    // Then: the sun and two lamps, in tree order
    let scene = rig.scene.borrow();
    let kinds: Vec<_> = scene.lights().into_iter().map(|(_, light)| light).collect();
    assert!(matches!(kinds[0], Light::Directional { .. }));
    assert!(matches!(kinds[2], Light::Point { range, .. } if (range - 5.0).abs() < 1e-6));
    assert_eq!(kinds.len(), 3);
    assert_eq!(scene.ambient(), ambient);
}

#[test]
fn a_transform_change_moves_that_light_and_recomputes_it_alone() {
    // Given
    let rig = LightsRig::new(1, Ambient::NONE);
    rig.scene.borrow_mut().update();

    // When
    rig.place.set(at(2.0));

    // Then
    let mut scene = rig.scene.borrow_mut();
    let sun = scene.lights()[0].0;
    assert_eq!(scene.update(), [sun]);
    assert_eq!(scene.world(sun).unwrap().w_axis.truncate(), Vec3::X * 2.0);
}

#[test]
#[should_panic(expected = "the scene already holds 8 lights")]
fn a_ninth_light_is_refused_at_mount_naming_the_limit() {
    // Given / When: the sun and eight lamps
    LightsRig::new(8, Ambient::NONE);
}

#[test]
fn eight_lights_mount() {
    // Given / When
    let rig = LightsRig::new(7, Ambient::NONE);

    // Then
    assert_eq!(rig.scene.borrow().lights().len(), 8);
}
