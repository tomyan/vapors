//! One-scene S5a: the table the build emits for the corpus's literal
//! shapes passes its own check.

use vapors_elements_corpus::SHAPES;

#[test]
fn the_emitted_table_passes_its_check() {
    SHAPES.check().unwrap_or_else(|error| panic!("{error}"));
}
