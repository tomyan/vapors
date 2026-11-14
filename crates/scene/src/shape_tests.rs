//! A shape ref's two kinds, each naming itself.

use super::{RuntimeRef, ShapeRef};

#[test]
fn each_ref_names_its_kind() {
    let runtime = ShapeRef::Runtime(RuntimeRef {
        document: 3,
        index: 2,
        generation: 5,
    });
    assert_eq!(
        runtime.to_string(),
        "run-time shape 2 (generation 5) of document 3"
    );
    assert_eq!(ShapeRef::Table(2).to_string(), "shape 2");
}
