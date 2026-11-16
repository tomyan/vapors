use super::*;

fn triangle() -> Vec<Vec3> {
    vec![Vec3::ZERO, Vec3::X, Vec3::Y]
}

#[test]
fn each_position_keeps_the_shade_given_for_it() {
    // Given
    let mut meshes = Meshes::new();

    // When
    let handle = meshes
        .geometry(triangle(), vec![0, 1, 2], vec![0.65, 1.0, 0.8])
        .unwrap();

    // Then
    let geometry = meshes.get(handle.id()).expect("held");
    assert_eq!(geometry.shades(), &[0.65, 1.0, 0.8]);
}

#[test]
fn a_shade_count_other_than_the_position_count_is_refused_naming_both() {
    for shades in [vec![], vec![1.0, 1.0], vec![1.0; 4]] {
        // When
        let refused = Meshes::new().geometry(triangle(), vec![0, 1, 2], shades.clone());

        // Then
        let count = shades.len();
        assert_eq!(
            refused,
            Err(MeshError::ShadeCount {
                shades: count,
                positions: 3
            })
        );
        let message = refused.unwrap_err().to_string();
        assert!(
            message.contains(&format!("{count} shades")) && message.contains("3 positions"),
            "{message}"
        );
    }
}

#[test]
fn a_shade_outside_0_to_1_or_not_a_number_is_refused_naming_it() {
    for (bad, shade) in [(1, 1.5), (2, -0.1), (0, f32::NAN)] {
        // Given
        let mut shades = vec![1.0; 3];
        shades[bad] = shade;

        // When
        let refused = Meshes::new().geometry(triangle(), vec![0, 1, 2], shades);

        // Then
        let message = refused.expect_err("refused").to_string();
        assert!(
            message.starts_with(&format!("shade {bad} is {shade}")),
            "{message}"
        );
    }
}
