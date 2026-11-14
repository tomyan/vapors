//! A scene anchor's used style is its `SceneUsed` (docs/plans/css-for-3d.md
//! C2): the appearance a node reads, from the tag's property set in
//! vapors-css. A declaration of any other longhand is refused at the
//! update naming the declaration, the property and what to use instead;
//! a value inherited from HTML is no declaration and is not refused.

mod support;

use support::{document, element, refusals, viewport};
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Cursor, Document, SceneUsed};
use vapors_scene::Color;

/// A body holding a `scene.stage` > `group.craft` > `mesh.body`, with
/// `css` added as `app.css` after a first line; the three anchors.
fn scene(css: &str) -> (Document, [NodeId; 3]) {
    let mut doc = document();
    let body = doc.body();
    let scene = element(&mut doc, body, "scene", "stage");
    let group = element(&mut doc, scene, "group", "craft");
    let mesh = element(&mut doc, group, "mesh", "body");
    doc.add_stylesheet("app.css", &format!("body {{ margin: 0; }}\n{css}"))
        .expect("parses");
    (doc, [scene, group, mesh])
}

fn updated(doc: &mut Document) {
    doc.update(viewport(400.0, 300.0)).expect("no refusal");
}

fn scene_used(doc: &Document, node: NodeId) -> SceneUsed {
    doc.scene_used(node)
        .expect("a scene anchor has a SceneUsed")
}

#[test]
fn a_box_longhand_on_a_mesh_is_refused_naming_the_rule_and_the_alternative() {
    // Given
    let (mut doc, _) = scene("mesh { width: 10px; }");

    // When
    let refused = refusals(&mut doc).join("\n");

    // Then
    assert!(
        refused.contains("app.css:2:")
            && refused.contains("<mesh class=\"body\">")
            && refused.contains("`width: 10px`")
            && refused.contains("<Mesh geometry>"),
        "{refused}"
    );
}

#[test]
fn opacity_on_a_group_is_refused_naming_the_meshes() {
    // Given
    let (mut doc, _) = scene(".craft { opacity: 0.5; }");

    // When
    let refused = refusals(&mut doc).join("\n");

    // Then
    assert!(
        refused.contains("<group class=\"craft\">")
            && refused.contains("`opacity: 0.5`")
            && refused.contains("each `mesh`"),
        "{refused}"
    );
}

#[test]
fn a_style_attribute_on_a_mesh_is_checked_as_a_rule_is() {
    // Given
    let (mut doc, [_, _, mesh]) = scene("");
    doc.set_attribute(mesh, "style", "margin-top: 2px");

    // When
    let refused = refusals(&mut doc).join("\n");

    // Then
    assert!(
        refused.contains("style attribute") && refused.contains("`margin-top: 2px`"),
        "{refused}"
    );
}

#[test]
fn a_transition_on_a_mesh_is_refused_until_c7() {
    // Given
    let (mut doc, _) = scene(".body { transition-duration: 200ms; }");

    // When
    let refused = refusals(&mut doc).join("\n");

    // Then
    assert!(
        refused.contains("`transition-duration: 200ms`") && refused.contains("C7"),
        "{refused}"
    );
}

#[test]
fn values_inherited_from_html_above_a_scene_refuse_nothing() {
    // Given: inherited text and box-ish values reach every anchor
    let (mut doc, _) = scene("body { font-size: 14px; line-height: 2; cursor: pointer; }");

    // Then
    updated(&mut doc);
}

#[test]
fn opacity_on_a_div_is_still_refused() {
    // Given
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, "div", "panel");
    doc.add_stylesheet("app.css", ".panel { opacity: .5; }")
        .expect("parses");

    // When
    let refused = refusals(&mut doc).join("\n");

    // Then
    assert!(
        refused.contains("`opacity: 0.5`") && refused.contains("F3"),
        "{refused}"
    );
}

#[test]
fn a_mesh_reads_its_colour_opacity_visibility_pointer_events_and_cursor() {
    // Given
    let (mut doc, [.., mesh]) = scene(
        ".body { color: #ff0000; opacity: 50%; visibility: hidden; pointer-events: none; cursor: pointer; }",
    );

    // When
    updated(&mut doc);

    // Then
    let used = scene_used(&doc, mesh);
    assert_eq!(used.colour, Color::from_rgba8(255, 0, 0, 255));
    assert!((used.opacity - 0.5).abs() < f32::EPSILON, "{used:?}");
    assert!(!used.visible && !used.takes_pointer, "{used:?}");
    assert_eq!(used.cursor, Cursor::Pointer);
}

#[test]
fn a_mesh_with_no_rule_takes_the_colour_its_ancestors_give_it() {
    // Given: `color` inherits from the body through the anchors
    let (mut doc, [.., mesh]) = scene("body { color: #0000ff; }");

    // When
    updated(&mut doc);

    // Then
    let used = scene_used(&doc, mesh);
    assert_eq!(used.colour, Color::from_rgba8(0, 0, 255, 255));
    assert!((used.opacity - 1.0).abs() < f32::EPSILON && used.visible && used.takes_pointer);
}

#[test]
fn visibility_on_a_group_reaches_its_meshes() {
    // Given
    let (mut doc, [_, group, mesh]) = scene(".craft { visibility: hidden; }");

    // When
    updated(&mut doc);

    // Then
    assert!(!scene_used(&doc, group).visible);
    assert!(!scene_used(&doc, mesh).visible);
}

#[test]
fn an_element_that_is_no_scene_anchor_has_no_scene_used() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let div = element(&mut doc, body, "div", "panel");

    // When
    updated(&mut doc);

    // Then
    assert_eq!(doc.scene_used(div), None);
}
