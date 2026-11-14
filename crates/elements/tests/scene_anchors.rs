//! The anchors of the 3D scene components (`vapors-scene3d-components`,
//! docs/plans/css-for-3d.md C1): `scene`, `group`, `mesh`, `camera` and
//! `decal` elements, which the user-agent sheet gives `display: none`.
//! A scene node is drawn by its surface, never laid out, so any other
//! `display` is refused naming the declaration.

mod support;

use support::{document, element, refusals, viewport};
use vapors_css::property::scene::SceneTag;
use vapors_elements::Document;

/// Every scene tag, from the one list vapors-css keeps.
fn scene_tags() -> impl Iterator<Item = &'static str> {
    SceneTag::ALL.into_iter().map(SceneTag::name)
}

/// A body holding a `tag.node` element, with `css` added as `app.css`.
fn page(tag: &str, css: &str) -> Document {
    let mut doc = document();
    let body = doc.body();
    element(&mut doc, body, tag, "node");
    doc.add_stylesheet("app.css", &format!("body {{ margin: 0; }}\n{css}"))
        .expect("parses");
    doc
}

#[test]
fn a_scene_anchor_is_implemented_and_builds_no_box() {
    for tag in scene_tags() {
        // Given
        let mut doc = page(tag, ".node { --tint: #ff0000; }");

        // When
        let updated = doc.update(viewport(400.0, 300.0)).map(|_| ());

        // Then: the root and the body, and no box for the anchor
        assert!(updated.is_ok(), "<{tag}>: {updated:?}");
        assert_eq!(doc.update_counts().boxes_built, 2, "<{tag}>");
    }
}

#[test]
fn a_display_other_than_none_on_a_scene_anchor_is_refused_naming_the_declaration() {
    for tag in scene_tags() {
        // Given
        let mut doc = page(tag, ".node { display: block; }");

        // When
        let refused = refusals(&mut doc).join("\n");

        // Then: the declaration on line 2, the element and the property
        assert!(
            refused.contains("app.css:2:")
                && refused.contains(&format!("<{tag} class=\"node\">"))
                && refused.contains("`display: block`")
                && refused.contains("scene node"),
            "<{tag}>: {refused}"
        );
    }
}

#[test]
fn a_display_on_an_anchor_inside_a_hidden_anchor_is_refused_too() {
    // Given: a `group.craft` inside a `scene`, which is `display: none`
    let mut doc = document();
    let body = doc.body();
    let scene = element(&mut doc, body, "scene", "stage");
    element(&mut doc, scene, "group", "craft");
    doc.add_stylesheet("app.css", "body { margin: 0; }\n.craft { display: block; }")
        .expect("parses");

    // When
    let refused = refusals(&mut doc).join("\n");

    // Then
    assert!(
        refused.contains("app.css:2:")
            && refused.contains("<group class=\"craft\">")
            && refused.contains("`display: block`"),
        "{refused}"
    );
}

#[test]
fn the_user_agent_sheet_gives_every_scene_tag_display_none() {
    for tag in scene_tags() {
        // Given
        let mut doc = page(tag, "");
        let node = doc.children(doc.body())[0];

        // When
        doc.update(viewport(400.0, 300.0)).expect("no refusal");

        // Then
        let style = doc.computed_style(node).expect("styled");
        let display = style.get(vapors_css::property::longhands::DISPLAY);
        assert_eq!(display.as_keyword(), Some("none"), "<{tag}>");
    }
}
