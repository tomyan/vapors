//! States no call can reach, forged by writing the document's fields:
//! a watched surface whose box the last update did not keep, and the
//! surface index naming a scene without a source.

use vapors_dom::{Dom, NodeId};
use vapors_text::FontSystem;

use crate::{Document, Viewport};

/// A watched 100 x 100 surface over a scene with one mesh, updated,
/// whose source answers the mesh.
fn watched() -> (Document, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    let body = doc.body();
    let surface = doc.create_element("gpu-surface");
    doc.set_attribute(surface, "name", "view");
    doc.append(body, surface);
    let scene = doc.create_element("scene");
    let mesh = doc.create_element("mesh");
    doc.append(body, scene);
    doc.append(scene, mesh);
    let css = "body { margin: 0; } gpu-surface { display: block; width: 100px; height: 100px; }";
    doc.add_stylesheet("app.css", css).expect("parses");
    doc.watch_hits(scene, "view", Box::new(move |_| Ok(Some(mesh))));
    let viewport = Viewport {
        width: 200.0,
        height: 200.0,
        scale: 1.0,
    };
    doc.update(viewport).expect("updates");
    (doc, surface)
}

#[test]
fn a_watched_surface_whose_box_was_not_kept_is_refused_not_passed_through() {
    // Given: the surface's used style forged away
    let (mut doc, surface) = watched();
    doc.kept.as_mut().expect("kept").used.remove(&surface);

    // When
    let hit = doc.hit([50.0, 50.0]);

    // Then
    assert_eq!(hit, Some(surface));
    let refused = doc.scene_hits.refused.borrow();
    assert!(
        refused
            .iter()
            .any(|e| e.to_string().contains("could not be read")),
        "{refused:?}"
    );
}

#[test]
#[should_panic(expected = "inserted and removed together")]
fn the_surface_index_naming_a_scene_without_a_source_panics_naming_the_invariant() {
    // Given: the source forged away, the index kept
    let (mut doc, _) = watched();
    doc.scene_hits.sources = crate::node_map::NodeMap::default();

    // When
    let _ = doc.hit([50.0, 50.0]);
}
