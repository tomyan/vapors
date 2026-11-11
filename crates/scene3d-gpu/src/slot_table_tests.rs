use super::*;
use vapors_scene3d::{NodeKind, Scene, Transform};

fn group(scene: &mut Scene) -> NodeId {
    let root = scene.root();
    scene
        .add(root, NodeKind::Group, Transform::IDENTITY)
        .unwrap()
}

#[test]
fn adding_and_removing_a_node_many_times_needs_no_more_slots_than_are_live() {
    // Given: one node that stays
    let mut scene = Scene::new();
    let mut table = SlotTable::default();
    let kept = group(&mut scene);

    // When: a second node is added and removed 100 times
    let needed: Vec<u32> = (0..100)
        .map(|_| {
            let churned = group(&mut scene);
            let needed = table.assign(&[kept, churned]);
            scene.remove(churned).unwrap();
            needed
        })
        .collect();

    // Then
    assert!(needed.iter().all(|&n| n == 2), "{needed:?}");
}

#[test]
fn a_reused_slot_is_unwritten() {
    // Given: a node written at version 1, then removed
    let mut scene = Scene::new();
    let mut table = SlotTable::default();
    let removed = group(&mut scene);
    table.assign(&[removed]);
    let index = table.mark_written(removed, 1);
    let added = group(&mut scene);

    // When
    table.assign(&[added]);

    // Then
    assert_eq!(table.index(added), index);
    assert!(!table.holds(added, 1));
    assert_eq!(table.index(removed), None);
}

#[test]
fn a_buffer_grows_to_the_next_power_of_two_cut_to_the_device_limit() {
    let cases = [
        (17, 256, 1 << 20, 32),
        (17, 256, 256 * 20, 20),
        (16, 256, 256 * 16, 16),
    ];
    for (needed, stride, limit, capacity) in cases {
        assert_eq!(
            capacity_for(needed, stride, limit),
            Ok(capacity),
            "{needed} slots of {stride} bytes under {limit}"
        );
    }
}

#[test]
fn slots_past_the_device_buffer_limit_are_refused_naming_count_and_limit() {
    // When
    let refused = capacity_for(17, 256, 256 * 16);

    // Then
    let error = refused.expect_err("refused");
    assert_eq!(
        error,
        FrameError::TooManySlots {
            slots: 17,
            stride: 256,
            limit: 4096,
        }
    );
    let message = error.to_string();
    assert!(message.contains("17 mesh uniform slots"), "{message}");
    assert!(
        message.contains("max_buffer_size of 4096 bytes"),
        "{message}"
    );
}
