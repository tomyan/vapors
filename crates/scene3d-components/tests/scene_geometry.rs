//! Geometry held by handles (`docs/plans/3d-2.md` row 5, decision (B)):
//! a page passing a `GeometryHandle` keeps its piece held while the page
//! lives, so a mesh removed and added again finds it; once the page is
//! unmounted and dropped and no other handle lives, the next sweep
//! releases it.

mod support;

use support::rows::RowsRig;

#[test]
fn a_row_dropped_and_added_again_finds_its_geometry_held() {
    // Given: one row, its piece held by the page's prop
    let rig = RowsRig::new(vec![1]);
    let id = square_id(&rig);

    // When
    rig.rows.set(Vec::new());
    let released = rig.stage.meshes.borrow_mut().release_unclaimed();
    rig.rows.set(vec![1]);

    // Then
    assert_eq!(released, []);
    assert!(rig.stage.meshes.borrow().holds(id));
    assert_eq!(
        rig.scene.borrow().len(),
        2 + 1,
        "root, group and the row's mesh"
    );
}

#[test]
fn an_unmounted_pages_geometry_is_released_at_the_next_sweep() {
    // Given
    let rig = RowsRig::new(vec![1, 2]);
    let id = square_id(&rig);
    let RowsRig { page, stage, .. } = rig;

    // When: `unmount` takes the page
    page.unmount(&stage.doc);
    let released = stage.meshes.borrow_mut().release_unclaimed();

    // Then
    assert_eq!(released, [id]);
}

/// The id of the piece the rows' meshes name.
fn square_id(rig: &RowsRig) -> vapors_scene3d::GeometryId {
    let scene = rig.scene.borrow();
    let group = scene.children(scene.root()).unwrap()[0];
    let mesh = scene.children(group).unwrap()[0];
    match scene.kind(mesh).unwrap() {
        vapors_scene3d::NodeKind::Mesh { geometry, .. } => geometry.id(),
        other => panic!("a mesh, found {other:?}"),
    }
}
