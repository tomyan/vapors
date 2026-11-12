#[test]
fn car_has_solid_height_and_local_transform_flags() {
    // Given / When
    let mesh = super::car();
    // Then
    assert_eq!(mesh.vertices.len() % 21, 0);
    assert!(
        mesh.vertices
            .as_chunks::<7>()
            .0
            .iter()
            .any(|v| v[2] >= 14.0)
    );
    assert!(
        mesh.vertices
            .as_chunks::<7>()
            .0
            .iter()
            .all(|v| v[6].to_bits() == 1.0_f32.to_bits())
    );
}
