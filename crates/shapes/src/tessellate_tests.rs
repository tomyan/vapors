//! Fills and strokes through lyon: the triangles cover the shape and
//! pass the mesh check; arcs flatten within the tolerance; a bad
//! tolerance or width and a shape without area are refused naming it.

use super::*;
use crate::mesh::{FAR_EDGE, check_mesh};

const FILL: Paint = Paint::Fill(FillRule::NonZero);
const TOLERANCE: f32 = 0.05;

fn mesh(data: &str, paint: Paint) -> Mesh {
    let path = IconPath::parse(data).unwrap_or_else(|e| panic!("{data:?}: {e}"));
    let mesh =
        tessellate("a.wisp:3", &path, paint, TOLERANCE).unwrap_or_else(|e| panic!("{data:?}: {e}"));
    assert_eq!(check_mesh(&mesh.triangles), Ok(()), "{data}");
    mesh
}

fn refused(data: &str, paint: Paint, tolerance: f32) -> String {
    let path = IconPath::parse(data).unwrap_or_else(|e| panic!("{data:?}: {e}"));
    tessellate("b.wisp:7", &path, paint, tolerance).expect_err("refused")
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
        .filter(|d| *d != FAR_EDGE)
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
    assert_eq!(mesh.tolerance, TOLERANCE);
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
fn a_filled_circle_of_arcs_covers_its_area_within_the_flattening() {
    // When: radius 6, through the tessellator (not the classifier)
    let mesh = mesh("M2 8a6 6 0 1 0 12 0a6 6 0 1 0-12 0z", FILL);

    // Then: pi r^2 = 113.097; chords at tolerance 0.05 lose at most
    // 2/3 x perimeter x tolerance = 1.26, and the cubic arcs add
    // under 2.7e-4 r = 0.002 (lyon's quadratics would add 0.7)
    let covered = area(&mesh);
    assert!(
        covered < 113.097 + 0.01 && covered > 113.097 - 1.26,
        "{covered}"
    );
    assert!(mesh.triangles.len() > 8);
    // and no vertex is further from the centre than the cubic's
    // 2.7e-4 r allows (a quadratic's vertices reach 1.0031 r = 6.019)
    let furthest = mesh
        .triangles
        .iter()
        .flat_map(|t| t.iter().map(|v| v.position))
        .map(|p| ((p[0] - 8.0).powi(2) + (p[1] - 8.0).powi(2)).sqrt())
        .fold(0.0f32, f32::max);
    assert!(furthest < 6.0 + 0.005, "{furthest}");
}

#[test]
fn an_arc_sweeps_in_the_direction_its_flag_says() {
    // When: sweep 1 is the positive-angle direction, which on a y-down
    // viewBox takes an arc from left to right over the top (y below
    // 0); sweep 0 under the bottom
    let over = mesh("M0 0A4 4 0 0 1 8 0z", FILL);
    let under = mesh("M0 0A4 4 0 0 0 8 0z", FILL);

    // Then
    let ys = |m: &Mesh| -> Vec<f32> {
        m.triangles
            .iter()
            .flat_map(|t| t.iter().map(|v| v.position[1]))
            .collect()
    };
    assert!(ys(&over).iter().all(|y| *y <= 1e-5), "{:?}", ys(&over));
    assert!(ys(&under).iter().all(|y| *y >= -1e-5), "{:?}", ys(&under));
    assert!(ys(&over).iter().any(|y| *y < -3.9));
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
fn a_pentagram_fills_its_centre_under_nonzero_and_not_under_evenodd() {
    // Given: a self-intersecting star
    let data = "M50 0L61 100L0 38L100 38L39 100Z";

    // When
    let non_zero = mesh(data, FILL);
    let even_odd = mesh(data, Paint::Fill(FillRule::EvenOdd));

    // Then: the inner pentagon is the difference
    assert!(
        area(&non_zero) > area(&even_odd) + 500.0,
        "{} {}",
        area(&non_zero),
        area(&even_odd)
    );
}

#[test]
fn a_tolerance_or_width_that_is_not_above_zero_is_refused_naming_the_shape() {
    // When, Then
    for tolerance in [0.0, -1.0, f32::NAN] {
        assert_eq!(
            refused(
                "M0 0a4 4 0 0 1 8 0",
                Paint::Stroke { width: 1.0 },
                tolerance
            ),
            format!(
                "the shape at b.wisp:7 has tolerance {tolerance}, which is not a finite number above zero; write a positive tolerance"
            )
        );
    }
    assert_eq!(
        refused("M0 0h4", Paint::Stroke { width: -2.0 }, TOLERANCE),
        "the shape at b.wisp:7 has stroke-width -2, which is not a finite number above zero; write a positive stroke-width"
    );
}

#[test]
fn a_shape_without_area_is_refused_naming_it() {
    // When, Then: a filled line, a stroked point
    assert_eq!(
        refused("M0 0L4 4", FILL, TOLERANCE),
        "the shape at b.wisp:7 has no area when filled; nothing would be drawn, so remove it or give it area"
    );
    assert_eq!(
        refused("M3 3", Paint::Stroke { width: 1.0 }, TOLERANCE),
        "the shape at b.wisp:7 has no area when stroked; nothing would be drawn, so remove it or give it area"
    );
}
