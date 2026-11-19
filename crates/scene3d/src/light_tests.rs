use crate::{
    Ambient, Colour, Light, MAX_LIGHTS, NodeKind, Scene, SceneError, StoreError, Transform,
};

fn white() -> Colour {
    Colour::srgb(1.0, 1.0, 1.0).unwrap()
}

fn point(intensity: f32, range: f32) -> NodeKind {
    NodeKind::Light(Light::Point {
        colour: white(),
        intensity,
        range,
    })
}

fn add(scene: &mut Scene, kind: NodeKind) -> Result<crate::NodeId, StoreError> {
    let root = scene.root();
    scene.add(root, kind, Transform::IDENTITY)
}

#[test]
fn a_ninth_light_is_refused_naming_the_limit() {
    // Given: eight lights
    let mut scene = Scene::new();
    for _ in 0..MAX_LIGHTS {
        add(&mut scene, point(1.0, 5.0)).unwrap();
    }

    // When
    let refused = add(&mut scene, point(1.0, 5.0)).unwrap_err();

    // Then
    assert_eq!(refused, StoreError::TooManyLights);
    assert!(refused.to_string().contains("8 lights"), "{refused}");
    assert_eq!(scene.lights().len(), 8);
}

#[test]
fn removing_a_light_makes_room_for_another() {
    // Given
    let mut scene = Scene::new();
    let ids: Vec<_> = (0..MAX_LIGHTS)
        .map(|_| add(&mut scene, point(1.0, 5.0)).unwrap())
        .collect();
    scene.remove(ids[3]).unwrap();

    // When / Then
    add(&mut scene, point(1.0, 5.0)).expect("room for one");
}

#[test]
fn a_light_that_cannot_shine_is_refused_naming_its_field() {
    // Given
    let mut scene = Scene::new();
    let cases = [
        (point(-1.0, 5.0), "Light::Point intensity"),
        (point(1.0, 0.0), "Light::Point range"),
        (point(f32::NAN, 5.0), "Light::Point intensity"),
    ];

    for (kind, field) in cases {
        // When
        let refused = add(&mut scene, kind).unwrap_err();

        // Then
        assert!(refused.to_string().contains(field), "{refused}");
    }
    assert!(matches!(
        Ambient::new(white(), -0.1),
        Err(SceneError::OutOfRange { .. })
    ));
}

#[test]
fn lights_come_in_tree_order_and_other_nodes_are_not_lights() {
    // Given
    let mut scene = Scene::new();
    let group = add(&mut scene, NodeKind::Group).unwrap();
    let a = add(&mut scene, point(1.0, 5.0)).unwrap();
    let b = scene
        .add(group, point(2.0, 5.0), Transform::IDENTITY)
        .unwrap();

    // When
    let ids: Vec<_> = scene.lights().iter().map(|(id, _)| *id).collect();

    // Then
    assert_eq!(ids, [a, b]);
}
