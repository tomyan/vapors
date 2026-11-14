use super::*;

fn triangle() -> Vec<Vec3> {
    vec![Vec3::ZERO, Vec3::X, Vec3::Y]
}

#[test]
fn given_geometry_is_held_under_its_id_for_every_reader() {
    // Given
    let mut meshes = Meshes::new();
    let handle = meshes.geometry(triangle(), vec![0, 1, 2]).unwrap();
    let id = handle.id();

    // When: two renderers read it
    let first = meshes.get(id).cloned();
    let second = meshes.get(id).cloned();

    // Then
    let geometry = first.expect("held");
    assert_eq!(geometry.positions(), &triangle()[..]);
    assert_eq!(geometry.indices(), &[0, 1, 2]);
    assert_eq!(second, Some(geometry));
}

#[test]
fn geometry_from_another_meshes_is_not_found_in_this_one() {
    // Given
    let ours = Meshes::new();
    let foreign = Meshes::new()
        .geometry(triangle(), vec![0, 1, 2])
        .unwrap()
        .id();

    // Then
    assert_eq!(ours.get(foreign), None);
}

#[test]
fn an_id_from_another_meshes_is_not_issued_by_this_one() {
    // Given
    let mut ours = Meshes::new();
    let mut theirs = Meshes::new();
    let mine = ours.geometry(triangle(), vec![0, 1, 2]).unwrap().id();
    let foreign = theirs.geometry(triangle(), vec![0, 1, 2]).unwrap().id();

    // Then
    assert!(ours.issued(mine));
    assert!(!ours.issued(foreign));
}

#[test]
fn geometry_the_scene_cannot_draw_is_refused_and_not_held() {
    let cases: [(Vec<Vec3>, Vec<u32>, MeshError); 6] = [
        (vec![], vec![0, 1, 2], MeshError::NoPositions),
        (
            vec![Vec3::ZERO, Vec3::X],
            vec![0, 1, 1],
            MeshError::TooFewPositions { count: 2 },
        ),
        (triangle(), vec![], MeshError::NoIndices),
        (triangle(), vec![0, 1], MeshError::NotTriangles { count: 2 }),
        (
            triangle(),
            vec![0, 1, 3],
            MeshError::IndexOutOfRange {
                at: 2,
                index: 3,
                positions: 3,
            },
        ),
        (
            vec![Vec3::ZERO, Vec3::new(f32::INFINITY, 0.0, 0.0), Vec3::Y],
            vec![0, 1, 2],
            MeshError::NonFinite { at: 1 },
        ),
    ];
    for (positions, indices, refusal) in cases {
        // Given
        let mut meshes = Meshes::new();

        // When
        let result = meshes.geometry(positions, indices);

        // Then
        assert_eq!(result, Err(refusal));
        assert!(meshes.held.is_empty());
    }
}

#[test]
fn a_degenerate_triangle_is_accepted() {
    // Given
    let mut meshes = Meshes::new();

    // When
    let result = meshes.geometry(triangle(), vec![0, 0, 0]);

    // Then
    assert!(result.is_ok());
}

#[test]
fn geometry_past_the_portable_buffer_limit_is_refused_naming_limit_and_size() {
    // Given: the index buffer is the one over the limit
    let over = crate::mesh_error::MAX_BUFFER_BYTES / 4 + 3;

    // When
    let refused = crate::mesh_error::check_sizes(3, over);

    // Then
    assert_eq!(
        refused,
        Err(MeshError::BufferTooLarge {
            buffer: "index",
            bytes: over * 4,
            limit: crate::mesh_error::MAX_BUFFER_BYTES,
        })
    );
    assert!(refused.unwrap_err().to_string().contains("268435456"));
}

#[test]
fn a_position_buffer_past_the_portable_limit_is_refused() {
    // Given
    let over = crate::mesh_error::MAX_BUFFER_BYTES / 12 + 1;

    // When
    let refused = crate::mesh_error::check_sizes(over, 3);

    // Then
    assert_eq!(
        refused,
        Err(MeshError::BufferTooLarge {
            buffer: "position",
            bytes: over * 12,
            limit: crate::mesh_error::MAX_BUFFER_BYTES,
        })
    );
}

#[test]
fn buffers_exactly_at_the_limit_pass_the_size_check() {
    // When
    let result = crate::mesh_error::check_sizes(crate::mesh_error::MAX_BUFFER_BYTES / 12, 3);

    // Then
    assert_eq!(result, Ok(()));
}
