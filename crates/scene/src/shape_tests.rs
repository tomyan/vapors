//! A shape ref's two ranges: a run-time ref carries its cache index
//! with the run-time bit, a table ref none; each names itself.

use super::ShapeRef;

#[test]
fn a_runtime_ref_carries_its_index_and_a_table_ref_none() {
    assert_eq!(ShapeRef::runtime(5).runtime_index(), Some(5));
    assert_eq!(ShapeRef(5).runtime_index(), None);
    assert_ne!(ShapeRef::runtime(5), ShapeRef(5));
}

#[test]
fn each_ref_names_its_range() {
    assert_eq!(ShapeRef::runtime(2).to_string(), "run-time shape 2");
    assert_eq!(ShapeRef(2).to_string(), "shape 2");
}

#[test]
#[should_panic(expected = "run-time shape 2147483648 is past the run-time range")]
fn an_index_past_the_runtime_range_is_a_bug() {
    let _ = ShapeRef::runtime(ShapeRef::RUNTIME);
}
