use super::*;
use crate::docs_patterns::{found, pieces};

#[test]
fn string_literals_joined_by_plus_are_read_as_one() {
    // Given: a message written as JavaScript literals joined across lines
    let source = Source::new(
        "first_run.js",
        "var TEXT = \"needs your own key. \"\n    + \"You pay the provider, \"\n    + 'and only it.';",
    );

    // Then
    let quote = "needs your own key. You pay the provider, and only it.";
    assert!(found(&pieces(quote), &[source]));
}
