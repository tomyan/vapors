use super::*;

fn triangle() -> Vec<Vec3> {
    vec![Vec3::ZERO, Vec3::X, Vec3::Y]
}

fn lit(normals: Vec<Vec3>) -> Result<GeometryHandle, MeshError> {
    Meshes::new().lit_geometry(triangle(), vec![0, 1, 2], vec![1.0; 3], normals)
}

#[test]
fn normals_given_are_kept_and_plain_geometry_has_none() {
    let mut meshes = Meshes::new();
    let plain = meshes
        .geometry(triangle(), vec![0, 1, 2], vec![1.0; 3])
        .unwrap();
    let with = meshes
        .lit_geometry(triangle(), vec![0, 1, 2], vec![1.0; 3], vec![Vec3::Z; 3])
        .unwrap();
    assert_eq!(meshes.get(plain.id()).unwrap().normals(), None);
    assert_eq!(
        meshes.get(with.id()).unwrap().normals(),
        Some(&[Vec3::Z; 3][..])
    );
}

#[test]
fn a_normal_count_other_than_the_position_count_is_refused_naming_the_geometry() {
    let error = lit(vec![Vec3::Z; 2]).unwrap_err().to_string();
    assert!(error.contains("geometry 0 of meshes"), "{error}");
    assert!(error.contains("2 normals for 3 positions"), "{error}");
}

#[test]
fn a_normal_not_finite_is_refused_naming_the_vertex() {
    for bad in [f32::NAN, f32::INFINITY] {
        let error = lit(vec![Vec3::Z, Vec3::Z, Vec3::new(bad, 0.0, 0.0)]).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("vertex 2") && message.contains("NaN or infinite"),
            "{message}"
        );
    }
}

#[test]
fn a_normal_not_unit_length_is_refused_naming_the_vertex_and_length() {
    for (vertex, normal) in [(0, Vec3::ZERO), (1, Vec3::Z * 1.01), (2, Vec3::Z * 0.99)] {
        let mut normals = vec![Vec3::Z; 3];
        normals[vertex] = normal;
        let message = lit(normals).unwrap_err().to_string();
        assert!(
            message.contains(&format!("vertex {vertex} has length")),
            "{message}"
        );
        assert!(message.contains("normalise"), "{message}");
    }
}

#[test]
fn a_normal_within_the_tolerance_is_accepted() {
    assert!(lit(vec![Vec3::Z * 1.0009, Vec3::Z * 0.9991, Vec3::Z]).is_ok());
}

/// The CPU cross product of a triangle's corners, normalised.
fn cross(positions: &[Vec3], corners: &[u32]) -> Vec3 {
    let [a, b, c] = [0, 1, 2].map(|k| positions[corners[k] as usize]);
    (b - a).cross(c - a).normalize()
}

/// 8 corners of the unit box, 12 counter-clockwise triangles seen from outside.
fn unit_box() -> (Vec<Vec3>, Vec<u32>) {
    let p = (0..8u8).map(|i| {
        Vec3::new(
            f32::from(i & 1),
            f32::from((i >> 1) & 1),
            f32::from((i >> 2) & 1),
        )
    });
    let indices = [
        0, 2, 1, 1, 2, 3, 4, 5, 6, 5, 7, 6, 0, 1, 4, 1, 5, 4, 2, 6, 3, 3, 6, 7, 0, 4, 2, 2, 4, 6,
        1, 3, 5, 3, 7, 5,
    ];
    (p.collect(), indices.to_vec())
}

fn prism() -> (Vec<Vec3>, Vec<u32>) {
    let positions = vec![
        Vec3::ZERO,
        Vec3::X,
        Vec3::Y,
        Vec3::new(0.0, 0.0, 2.0),
        Vec3::new(1.0, 0.0, 2.0),
        Vec3::new(0.0, 1.0, 2.0),
    ];
    let indices = vec![
        0, 2, 1, 3, 4, 5, 0, 1, 4, 0, 4, 3, 1, 2, 5, 1, 5, 4, 2, 0, 3, 2, 3, 5,
    ];
    (positions, indices)
}

#[test]
fn flat_normals_match_the_cpu_cross_product_for_a_box_and_a_prism() {
    for (positions, indices) in [unit_box(), prism()] {
        let flat = flat_shaded(&positions, &indices, &vec![1.0; positions.len()]).unwrap();
        assert_eq!(flat.positions.len(), indices.len());
        for (t, corners) in indices.chunks(3).enumerate() {
            for k in 0..3 {
                let at = t * 3 + k;
                assert_eq!(flat.positions[at], positions[corners[k] as usize]);
                assert!(flat.normals[at].abs_diff_eq(cross(&positions, corners), 1e-6));
            }
        }
        assert_eq!(
            flat.indices,
            (0..u32::try_from(indices.len()).unwrap()).collect::<Vec<_>>()
        );
    }
}

#[test]
fn the_box_faces_point_outward() {
    let (positions, indices) = unit_box();
    let flat = flat_shaded(&positions, &indices, &[1.0; 8]).unwrap();
    for (corner, normal) in flat.positions.iter().zip(&flat.normals) {
        assert!(
            (*corner - Vec3::splat(0.5)).dot(*normal) > 0.0,
            "{corner} {normal}"
        );
    }
}

#[test]
fn flat_shading_keeps_each_corners_shade() {
    let shades = [0.1, 0.5, 0.9];
    let flat = flat_shaded(&triangle(), &[2, 0, 1], &shades).unwrap();
    assert_eq!(flat.shades, vec![0.9, 0.1, 0.5]);
}

#[test]
fn a_degenerate_triangle_is_refused_naming_it() {
    let error = flat_shaded(&triangle(), &[0, 1, 2, 0, 1, 1], &[1.0; 3]).unwrap_err();
    assert_eq!(
        error,
        MeshError::Normals(NormalError::Degenerate { triangle: 1 })
    );
    assert!(error.to_string().contains("triangle 1"));
}

#[test]
fn flat_shading_refuses_what_geometry_refuses() {
    assert_eq!(
        flat_shaded(&triangle(), &[0, 1, 3], &[1.0; 3]).unwrap_err(),
        MeshError::IndexOutOfRange {
            at: 2,
            index: 3,
            positions: 3
        }
    );
    assert!(matches!(
        flat_shaded(&triangle(), &[0, 1, 2], &[1.0; 2]),
        Err(MeshError::ShadeCount { .. })
    ));
}

#[test]
fn flat_geometry_holds_the_flat_triangles() {
    let mut meshes = Meshes::new();
    let (positions, indices) = unit_box();
    let handle = meshes
        .flat_geometry(&positions, &indices, &[1.0; 8])
        .unwrap();
    let geometry = meshes.get(handle.id()).unwrap();
    assert_eq!(geometry.positions().len(), 36);
    assert_eq!(geometry.normals().unwrap().len(), 36);
}
