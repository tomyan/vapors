//! GPU tier: mesh items drawn by the painter (one-scene S3a2): fill and
//! stroke meshes in paint order with closed-form shapes; each mesh
//! entry's vertices uploaded once, counted as `shape_bytes_uploaded`, so
//! an unchanged frame uploads and writes nothing.

#[path = "support/surface.rs"]
mod support;

use support::{BLUE, RED, Rig, WHITE, document, pixel, update};
use vapors_elements::test_items::paint_after_background;
use vapors_scene::{Color, Item, Rect, Shape, ShapeKind, ShapeRef};
use vapors_shapes::{
    FillRule, Geometry, IconPath, MeshVertex, Paint, ShapeEntry, ShapeTable, TABLE_FORMAT,
    hash_bytes, tessellate,
};

/// `data` painted by `paint`, tessellated, for the test binary's life.
fn mesh(data: &str, paint: Paint) -> &'static [[MeshVertex; 3]] {
    let path = IconPath::parse(data).expect("parses");
    let mesh = tessellate("gpu_shape_meshes.rs", &path, paint, 0.05).expect("tessellated");
    Box::leak(mesh.triangles.into_boxed_slice())
}

/// Shape 0: a 12 x 8 rect, closed form. Shape 1: an 8 x 8 square, a
/// filled mesh. Shape 2: a 12 wide line stroked 2 wide, a mesh.
fn table() -> ShapeTable {
    let entry = |site, canonical: &'static [u8], geometry| ShapeEntry {
        site,
        canonical,
        hash: hash_bytes(canonical),
        geometry,
    };
    let rect = Geometry::Rect {
        origin: [0.0, 0.0],
        size: [12.0, 8.0],
        radius: [0.0, 0.0],
    };
    let square = mesh("M0 0h8v8h-8z", Paint::Fill(FillRule::NonZero));
    let line = mesh("M0 1h12", Paint::Stroke { width: 2.0 });
    let entries = vec![
        entry("gpu_shape_meshes.rs:1", b"Frect", rect),
        entry(
            "gpu_shape_meshes.rs:2",
            b"Fsquare",
            Geometry::Mesh { triangles: square },
        ),
        entry(
            "gpu_shape_meshes.rs:3",
            &[b'S', 0, 0, 0, 64, b'l'],
            Geometry::Mesh { triangles: line },
        ),
    ];
    ShapeTable::new(
        "gpu_shape_meshes",
        TABLE_FORMAT,
        Box::leak(entries.into_boxed_slice()),
    )
}

/// Shape `shape` of `kind` moved to `at` in `colour`, its bounds `size`
/// from there.
fn item(shape: u32, kind: ShapeKind, at: [f32; 2], size: [f32; 2], colour: [u8; 4]) -> Item {
    Item::Shape(Shape {
        shape: ShapeRef(shape),
        kind,
        transform: [1.0, 0.0, 0.0, 1.0, at[0], at[1]],
        colour: Color::from_rgba8(colour[0], colour[1], colour[2], colour[3]),
        bounds: Rect::new(at[0], at[1], at[0] + size[0], at[1] + size[1]),
    })
}

/// A red rect at (0, 0), a blue mesh square over its right end at
/// (8, 0), a red rect over the square's right half at (14, 2), and a
/// blue stroked line along y = 24.
fn page() -> vapors_elements::Document {
    let mut doc = document(
        "div { position: absolute; width: 32px; height: 32px; }",
        &[(None, "div", "a")],
    );
    let a = doc.children(doc.body())[0];
    let (closed, mesh) = (ShapeKind::ClosedForm, ShapeKind::Mesh);
    paint_after_background(
        a,
        vec![
            item(0, closed, [0.0, 0.0], [12.0, 8.0], RED),
            item(1, mesh, [8.0, 0.0], [8.0, 8.0], BLUE),
            item(0, closed, [14.0, 2.0], [12.0, 8.0], RED),
            item(2, mesh, [0.0, 23.0], [12.0, 2.0], BLUE),
        ],
    );
    update(&mut doc, 1.0);
    doc
}

fn rig() -> Rig {
    let mut rig = Rig::new();
    rig.painter
        .shapes_mut()
        .register(&table())
        .expect("registers");
    rig
}

#[test]
fn gpu_meshes_are_drawn_in_paint_order_with_closed_forms() {
    // Given
    let doc = page();
    let mut rig = rig();

    // When
    let pixels = rig.draw(&doc).expect("paints");

    // Then
    assert_eq!(pixel(&pixels, 4, 4), RED, "the first rect");
    assert_eq!(pixel(&pixels, 10, 4), BLUE, "the mesh square over it");
    assert_eq!(
        pixel(&pixels, 15, 4),
        RED,
        "the second rect over the square"
    );
    assert_eq!(pixel(&pixels, 4, 24), BLUE, "the stroked line");
    assert_eq!(pixel(&pixels, 4, 27), WHITE, "below the line");
    let counts = rig.painter.frame_counts();
    assert_eq!((counts.shapes.draw_calls, counts.meshes.draw_calls), (2, 2));
}

#[test]
fn gpu_mesh_vertices_upload_once_and_an_unchanged_frame_writes_nothing() {
    // Given
    let doc = page();
    let mut rig = rig();
    let first = rig.draw(&doc).expect("paints");
    let uploaded = rig.painter.frame_counts().shape_bytes_uploaded;
    assert!(uploaded > 0, "the two meshes' vertices");

    // When
    let again = rig.draw(&doc).expect("paints");

    // Then
    let counts = rig.painter.frame_counts();
    assert_eq!(counts.shape_bytes_uploaded, 0, "{counts:?}");
    assert_eq!(counts.meshes.buffer_bytes + counts.shapes.buffer_bytes, 0);
    assert!(again == first, "the kept shapes draw the same pixels");
}
