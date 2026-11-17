use std::path::Path;

use super::{named, render_nodes};

#[test]
fn only_the_render_nodes_are_found_not_the_card_nodes() {
    // Given
    let dri = tempfile::tempdir().expect("a scratch directory");
    for name in ["card0", "renderD128", "by-path"] {
        std::fs::write(dri.path().join(name), "").expect("written");
    }

    // When
    let nodes = render_nodes(dri.path()).expect("read");

    // Then
    assert_eq!(nodes, [dri.path().join("renderD128")]);
}

#[test]
fn a_machine_with_no_gpu_has_no_render_nodes() {
    assert_eq!(render_nodes(Path::new("/nonexistent/dri")), Ok(Vec::new()));
}

#[test]
fn the_advisory_databases_are_the_entries_cargo_deny_names() {
    // Given
    let dbs = tempfile::tempdir().expect("a scratch directory");
    for name in ["advisory-db-3157", "db.lock"] {
        std::fs::write(dbs.path().join(name), "").expect("written");
    }

    // When
    let found = named(dbs.path(), "advisory-db-").expect("read");

    // Then
    assert_eq!(found, [dbs.path().join("advisory-db-3157")]);
}
