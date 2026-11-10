use super::*;
use crate::Vec3;

fn at(x: f32) -> Transform {
    Transform::from_translation(Vec3::new(x, 0.0, 0.0)).unwrap()
}

#[test]
fn an_added_node_sits_under_its_parent() {
    // Given
    let mut scene = Scene::new();
    let root = scene.root();

    // When
    let group = scene.add(root, NodeKind::Group, at(1.0)).unwrap();
    let mesh = scene.add(group, NodeKind::Mesh, at(2.0)).unwrap();

    // Then
    assert_eq!(scene.parent(mesh), Ok(Some(group)));
    assert_eq!(scene.children(root), Ok(&[group][..]));
    assert_eq!(scene.kind(mesh), Ok(&NodeKind::Mesh));
    assert_eq!(scene.len(), 3);
}

#[test]
fn a_removed_id_is_refused_after_its_slot_is_reused() {
    // Given: a node removed and another added into its slot
    let mut scene = Scene::new();
    let root = scene.root();
    let old = scene.add(root, NodeKind::Mesh, at(1.0)).unwrap();
    scene.remove(old).unwrap();
    let new = scene.add(root, NodeKind::Group, at(2.0)).unwrap();

    // When
    let result = scene.kind(old);

    // Then: the old id does not name the new node
    assert_eq!(new.index, old.index, "the slot is reused");
    assert_eq!(result, Err(StoreError::Stale(old)));
    assert_eq!(scene.kind(new), Ok(&NodeKind::Group));
}

#[test]
fn removing_a_node_releases_its_subtree() {
    // Given: root > group > (mesh, group > mesh), and a sibling mesh
    let mut scene = Scene::new();
    let root = scene.root();
    let group = scene.add(root, NodeKind::Group, at(1.0)).unwrap();
    let below = [
        scene.add(group, NodeKind::Mesh, at(1.0)).unwrap(),
        scene.add(group, NodeKind::Group, at(1.0)).unwrap(),
    ];
    let deepest = scene.add(below[1], NodeKind::Mesh, at(1.0)).unwrap();
    let sibling = scene.add(root, NodeKind::Mesh, at(1.0)).unwrap();

    // When
    let released = scene.remove(group).unwrap();

    // Then
    assert_eq!(released, vec![group, below[0], below[1], deepest]);
    for id in released {
        assert_eq!(scene.kind(id), Err(StoreError::Stale(id)));
    }
    assert_eq!(scene.children(root), Ok(&[sibling][..]));
    assert_eq!(scene.len(), 2);
}

#[test]
fn a_removed_id_is_refused_by_every_operation() {
    // Given
    let mut scene = Scene::new();
    let root = scene.root();
    let gone = scene.add(root, NodeKind::Group, at(1.0)).unwrap();
    scene.remove(gone).unwrap();
    let stale = Err(StoreError::Stale(gone));

    // When / Then
    assert_eq!(scene.add(gone, NodeKind::Mesh, at(1.0)), stale);
    assert_eq!(scene.remove(gone), Err(StoreError::Stale(gone)));
    assert_eq!(
        scene.set_transform(gone, at(2.0)),
        Err(StoreError::Stale(gone))
    );
    assert_eq!(scene.reparent(gone, root), Err(StoreError::Stale(gone)));
    assert_eq!(scene.world(gone), Err(StoreError::Stale(gone)));
}

#[test]
fn the_root_is_neither_removed_nor_moved() {
    // Given
    let mut scene = Scene::new();
    let root = scene.root();
    let group = scene.add(root, NodeKind::Group, at(1.0)).unwrap();

    // When / Then
    assert_eq!(scene.remove(root), Err(StoreError::RootFixed));
    assert_eq!(scene.reparent(root, group), Err(StoreError::RootFixed));
    assert_eq!(scene.len(), 2);
}

#[test]
fn store_errors_name_the_fix() {
    // Given
    let id = NodeId {
        scene: 0,
        index: 4,
        generation: 2,
    };
    let cases = [
        (StoreError::Stale(id), "node 4v2 was removed: use an id"),
        (
            StoreError::ForeignId(id),
            "use ids returned by the scene you call",
        ),
        (StoreError::RootFixed, "remove or move its children instead"),
        (StoreError::WorldPending(id), "call Scene::update"),
        (
            StoreError::Cycle {
                node: id,
                parent: id,
            },
            "pick a parent outside it",
        ),
    ];

    // When / Then
    for (error, expected) in cases {
        let message = error.to_string();
        assert!(message.contains(expected), "{message}");
    }
}

#[test]
fn a_slot_whose_generation_would_wrap_is_retired() {
    // Given: a node in a slot at the last generation
    let mut scene = Scene::new();
    let first = scene.add(scene.root(), NodeKind::Mesh, at(1.0)).unwrap();
    scene.slots[first.index as usize].generation = u32::MAX;
    let last = NodeId {
        scene: first.scene,
        index: first.index,
        generation: u32::MAX,
    };

    // When
    scene.remove(last).unwrap();
    let next = scene.add(scene.root(), NodeKind::Mesh, at(1.0)).unwrap();

    // Then: the slot is not reused, so no later id equals a stale one
    assert_ne!(next.index, last.index);
    assert_eq!(scene.kind(last), Err(StoreError::Stale(last)));
    assert_eq!(scene.len(), 2);
}

#[test]
fn an_id_from_another_scene_is_refused_by_every_operation() {
    // Given: two scenes whose first nodes share an index and generation
    let mut ours = Scene::new();
    let mut theirs = Scene::new();
    let mine = ours.add(ours.root(), NodeKind::Mesh, at(1.0)).unwrap();
    let foreign = theirs.add(theirs.root(), NodeKind::Mesh, at(1.0)).unwrap();
    assert_eq!(
        (foreign.index, foreign.generation),
        (mine.index, mine.generation)
    );
    let refused = StoreError::ForeignId(foreign);

    // When / Then
    assert_eq!(ours.kind(foreign), Err(refused.clone()));
    assert_eq!(
        ours.add(foreign, NodeKind::Mesh, at(1.0)),
        Err(refused.clone())
    );
    assert_eq!(ours.remove(foreign), Err(refused.clone()));
    assert_eq!(ours.set_transform(foreign, at(2.0)), Err(refused.clone()));
    assert_eq!(ours.reparent(mine, foreign), Err(refused.clone()));
    assert_eq!(ours.reparent(foreign, ours.root()), Err(refused.clone()));
    assert_eq!(
        ours.world(theirs.root()),
        Err(StoreError::ForeignId(theirs.root()))
    );
    assert_eq!(ours.len(), 2);
}
