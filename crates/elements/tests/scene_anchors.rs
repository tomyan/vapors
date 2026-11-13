//! The anchors of the 3D scene components (`vapors-scene3d-components`,
//! docs/plans/css-for-3d.md C1): `scene`, `group`, `mesh`, `camera` and
//! `decal` elements, which the user-agent sheet gives `display: none`.
//! A scene node is drawn by its surface, never laid out, so any other
//! `display` is refused naming the declaration.

mod support;

use support::{document, element, refusals, viewport};
use vapors_elements::Document;

const SCENE_TAGS: [&str; 5] = ["scene", "group", "mesh", "camera", "decal"];

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
    for tag in SCENE_TAGS {
        // Given
        let mut doc = page(tag, ".node { color: #ff0000; }");

        // When
        let updated = doc.update(viewport(400.0, 300.0)).map(|_| ());

        // Then: the root and the body, and no box for the anchor
        assert!(updated.is_ok(), "<{tag}>: {updated:?}");
        assert_eq!(doc.update_counts().boxes_built, 2, "<{tag}>");
    }
}

#[test]
fn a_display_other_than_none_on_a_scene_anchor_is_refused_naming_the_declaration() {
    for tag in SCENE_TAGS {
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
