//! A watched mesh anchor's appearance (`Dom::watch_appearance`,
//! docs/plans/css-for-3d.md C3): its computed `color` and `visibility`,
//! reported at the first update and at each update that changes them;
//! translucent values refused until C5; a sink's error refuses the
//! update; nothing reported once the anchor leaves the tree.

mod support;

use std::cell::RefCell;
use std::rc::Rc;
use support::{document, element, refusals, viewport};
use vapors_dom::{Appearance, Dom, NodeId};
use vapors_elements::Document;

type Seen = Rc<RefCell<Vec<Appearance>>>;

/// A `scene` > `mesh.body` with `css` after a first line; the mesh
/// watched into the returned list.
fn watched(css: &str) -> (Document, NodeId, Seen) {
    let seen = Seen::default();
    let into = Rc::clone(&seen);
    let sink = move |appearance| {
        into.borrow_mut().push(appearance);
        Ok(())
    };
    let (doc, mesh) = watched_by(css, Box::new(sink));
    (doc, mesh, seen)
}

/// The same page, the mesh watched by `sink`.
fn watched_by(css: &str, sink: vapors_dom::AppearanceSink) -> (Document, NodeId) {
    let mut doc = document();
    let body = doc.body();
    let scene = element(&mut doc, body, "scene", "stage");
    let mesh = element(&mut doc, scene, "mesh", "body");
    doc.add_stylesheet("app.css", &format!("body {{ margin: 0; }}\n{css}"))
        .expect("parses");
    doc.watch_appearance(mesh, sink);
    (doc, mesh)
}

fn updated(doc: &mut Document) {
    doc.update(viewport(400.0, 300.0)).expect("no refusal");
}

const RED: Appearance = Appearance {
    rgb: [1.0, 0.0, 0.0],
    visible: true,
};

#[test]
fn the_first_update_reports_the_computed_colour_and_visibility() {
    // Given
    let (mut doc, _, seen) = watched(".body { color: #ff0000; }");

    // When
    updated(&mut doc);

    // Then
    assert_eq!(*seen.borrow(), [RED]);
}

#[test]
fn an_update_that_changes_neither_reports_nothing() {
    // Given: the cursor changes, which is no part of the appearance
    let (mut doc, mesh, seen) = watched(".body { color: #ff0000; }\n.pointy { cursor: pointer; }");
    updated(&mut doc);

    // When
    doc.set_attribute(mesh, "class", "body pointy");
    updated(&mut doc);
    updated(&mut doc);

    // Then
    assert_eq!(*seen.borrow(), [RED]);
}

#[test]
fn a_class_change_reports_the_new_colour_and_visibility_once() {
    // Given
    let (mut doc, mesh, seen) =
        watched(".body { color: #ff0000; }\n.gone { color: #0000ff; visibility: hidden; }");
    updated(&mut doc);

    // When
    doc.set_attribute(mesh, "class", "body gone");
    updated(&mut doc);

    // Then
    let gone = Appearance {
        rgb: [0.0, 0.0, 1.0],
        visible: false,
    };
    assert_eq!(*seen.borrow(), [RED, gone]);
}

#[test]
fn opacity_under_one_on_a_watched_mesh_is_refused_until_c5() {
    // Given
    let (mut doc, _, _) = watched(".body { opacity: 0.5; }");

    // When
    let refused = refusals(&mut doc).join("\n");

    // Then
    assert!(
        refused.contains("app.css:2:")
            && refused.contains("`opacity: 0.5`")
            && refused.contains("C5"),
        "{refused}"
    );
}

#[test]
fn a_translucent_colour_inherited_by_a_watched_mesh_is_refused_at_the_mesh() {
    // Given: declared on the body, where text may be translucent
    let (mut doc, _, _) = watched("body { color: rgb(255 0 0 / 50%); }");

    // When
    let refused = refusals(&mut doc).join("\n");

    // Then
    assert!(
        refused.contains("<mesh class=\"body\">")
            && refused.contains("`color: ")
            && refused.contains("C5"),
        "{refused}"
    );
}

#[test]
fn a_sinks_error_refuses_the_update_naming_the_anchor() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let mesh = element(&mut doc, body, "mesh", "body");
    doc.watch_appearance(mesh, Box::new(|_| Err("the node was removed".to_owned())));

    // When
    let refused = refusals(&mut doc).join("\n");

    // Then
    assert!(
        refused.contains("<mesh class=\"body\">") && refused.contains("the node was removed"),
        "{refused}"
    );
}

#[test]
fn a_removed_anchors_sink_is_dropped() {
    // Given
    let (mut doc, mesh, seen) = watched("");
    updated(&mut doc);

    // When
    doc.remove(mesh);
    updated(&mut doc);

    // Then: the list's one holder is the test
    assert_eq!(Rc::strong_count(&seen), 1);
}

#[test]
fn watching_an_element_that_is_no_mesh_anchor_is_refused() {
    // Given
    let mut doc = document();
    let body = doc.body();
    let group = element(&mut doc, body, "group", "craft");
    doc.watch_appearance(group, Box::new(|_| Ok(())));

    // When
    let refused = refusals(&mut doc).join("\n");

    // Then
    assert!(
        refused.contains("<group class=\"craft\">") && refused.contains("watch_appearance"),
        "{refused}"
    );
}

#[test]
fn an_appearance_a_sink_refused_is_reported_again_at_the_next_update() {
    // Given: a sink that fails once
    let calls = Rc::new(std::cell::Cell::new(0));
    let counted = Rc::clone(&calls);
    let sink = move |_| {
        counted.set(counted.get() + 1);
        if counted.get() == 1 {
            Err("not yet".to_owned())
        } else {
            Ok(())
        }
    };
    let (mut doc, _) = watched_by("", Box::new(sink));
    refusals(&mut doc);

    // When: nothing else changed
    updated(&mut doc);

    // Then
    assert_eq!(calls.get(), 2);
}

#[test]
fn a_mesh_watched_after_its_first_update_reports_its_appearance() {
    // Given: styled before anyone watches it
    let mut doc = document();
    let body = doc.body();
    let mesh = element(&mut doc, body, "mesh", "body");
    doc.add_stylesheet("app.css", ".body { color: #ff0000; }")
        .expect("parses");
    updated(&mut doc);
    let seen = Seen::default();
    let into = Rc::clone(&seen);
    doc.watch_appearance(
        mesh,
        Box::new(move |appearance| {
            into.borrow_mut().push(appearance);
            Ok(())
        }),
    );

    // When
    updated(&mut doc);

    // Then
    assert_eq!(*seen.borrow(), [RED]);
}

#[test]
fn a_translucent_colour_made_opaque_by_an_opacity_above_one_is_refused_naming_opacity() {
    // Given: alpha 0.5 times opacity 2 would pass as opaque
    let (mut doc, _, seen) = watched(".body { color: rgb(255 0 0 / 50%); opacity: calc(1 + 1); }");

    // When
    let refused = refusals(&mut doc).join("\n");

    // Then
    assert!(
        refused.contains("`opacity: ") && refused.contains("0 to 1"),
        "{refused}"
    );
    assert!(seen.borrow().is_empty());
}

#[test]
fn a_translucent_colour_and_opacity_are_both_refused() {
    // Given
    let (mut doc, _, _) = watched(".body { color: rgb(255 0 0 / 50%); opacity: 0.5; }");

    // When
    let refused = refusals(&mut doc).join("\n");

    // Then
    assert!(
        refused.contains("`opacity: 0.5`") && refused.contains("`color: "),
        "{refused}"
    );
}

#[test]
fn a_second_watch_of_one_anchor_is_refused() {
    // Given
    let (mut doc, mesh, _) = watched("");

    // When
    doc.watch_appearance(mesh, Box::new(|_| Ok(())));
    let refused = refusals(&mut doc).join("\n");

    // Then
    assert!(refused.contains("already watched"), "{refused}");
}

#[test]
fn a_mesh_under_a_refused_anchor_style_reports_nothing_until_it_is_fixed() {
    // Given: drawn red; then the scene anchor declares a `color` it
    // does not read, which the mesh would inherit
    let (mut doc, mesh, seen) = watched(
        ".body { color: #ff0000; }\n.bad { color: #0000ff; }\n.bad mesh { color: inherit; }",
    );
    updated(&mut doc);
    let scene = doc.parent(mesh).expect("in the scene");
    doc.set_attribute(scene, "class", "stage bad");

    // When
    let refused = refusals(&mut doc).join("\n");
    doc.set_attribute(scene, "class", "stage");
    updated(&mut doc);

    // Then: the refused update reported nothing, and the fix nothing new
    assert!(refused.contains("<scene class=\"stage bad\">"), "{refused}");
    assert_eq!(*seen.borrow(), [RED]);
}
