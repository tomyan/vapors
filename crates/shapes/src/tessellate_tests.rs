//! Fills and strokes through lyon: the triangles cover the shape, the
//! outline edges carry distances; a shape without area is refused
//! naming it.

use super::*;
use crate::mesh_edges::FAR;

const FILL: Paint = Paint::Fill(FillRule::NonZero);

fn mesh(data: &str, paint: Paint) -> Mesh {
    let path = IconPath::parse(data).unwrap_or_else(|e| panic!("{data:?}: {e}"));
    tessellate("a.wisp:3", &path, paint, 0.05).unwrap_or_else(|e| panic!("{data:?}: {e}"))
}

/// The sum of the triangles' areas.
fn area(mesh: &Mesh) -> f32 {
    mesh.triangles
        .iter()
        .map(|[a, b, c]| {
            let (p, q, r) = (a.position, b.position, c.position);
            ((q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])).abs() / 2.0
        })
        .sum()
}

/// How many edges across the mesh are on the outline.
fn outline_edges(mesh: &Mesh) -> usize {
    mesh.triangles
        .iter()
        .flat_map(|t| t.iter().flat_map(|v| v.edges))
        .filter(|d| *d != FAR)
        .count()
        / 3
}

#[test]
fn a_filled_square_is_two_triangles_with_four_outline_edges() {
    // When
    let mesh = mesh("M0 0h4v4h-4z", FILL);

    // Then
    assert_eq!(mesh.triangles.len(), 2);
    assert!((area(&mesh) - 16.0).abs() < 1e-5, "{}", area(&mesh));
    assert_eq!(outline_edges(&mesh), 4);
}

#[test]
fn a_stroked_line_is_a_quad_of_its_width() {
    // When: a 10-unit line, 2 wide, butt caps
    let mesh = mesh("M0 0h10", Paint::Stroke { width: 2.0 });

    // Then
    assert_eq!(mesh.triangles.len(), 2);
    assert!((area(&mesh) - 20.0).abs() < 1e-4, "{}", area(&mesh));
    assert_eq!(outline_edges(&mesh), 4);
    let ys: Vec<f32> = mesh
        .triangles
        .iter()
        .flat_map(|t| t.iter().map(|v| v.position[1]))
        .collect();
    assert!(ys.iter().all(|y| (y.abs() - 1.0).abs() < 1e-5), "{ys:?}");
}

#[test]
fn a_filled_circle_of_arcs_covers_its_area_within_the_tolerance() {
    // When: radius 6, through the tessellator (not the classifier)
    let mesh = mesh("M2 8a6 6 0 1 0 12 0a6 6 0 1 0-12 0z", FILL);

    // Then: pi r^2 = 113.1; flattening at 0.05 loses under 1%
    let covered = area(&mesh);
    assert!((covered - 113.097).abs() < 1.2, "{covered}");
    assert!(mesh.triangles.len() > 8);
}

#[test]
fn even_odd_leaves_the_hole_nonzero_fills() {
    // Given: two squares, the inner wound the same way
    let data = "M0 0h8v8h-8zM2 2h4v4h-4z";

    // When
    let non_zero = mesh(data, FILL);
    let even_odd = mesh(data, Paint::Fill(FillRule::EvenOdd));

    // Then
    assert!((area(&non_zero) - 64.0).abs() < 1e-4, "{}", area(&non_zero));
    assert!((area(&even_odd) - 48.0).abs() < 1e-4, "{}", area(&even_odd));
}

#[test]
fn a_shape_without_area_is_refused_naming_it() {
    // Given: a filled line, a stroked point
    let line = IconPath::parse("M0 0L4 4").expect("parses");
    let dot = IconPath::parse("M3 3").expect("parses");

    // When, Then
    assert_eq!(
        tessellate("b.wisp:7", &line, FILL, 0.05),
        Err("the shape at b.wisp:7 has no area when filled; nothing would be drawn, so remove it or give it area".to_owned())
    );
    assert_eq!(
        tessellate("b.wisp:7", &dot, Paint::Stroke { width: 1.0 }, 0.05),
        Err("the shape at b.wisp:7 has no area when stroked; nothing would be drawn, so remove it or give it area".to_owned())
    );
}
