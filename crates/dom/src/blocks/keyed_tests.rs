//! A `{for}` whose rows share a key is refused naming the site, and
//! saying how its path is read: under the crate's directory, after the
//! crate's package name, so it names no path of the build machine.

use super::refuse_duplicate_keys;

#[test]
#[should_panic(
    expected = "vapors-components/src/list.wisp:5:5 (the .wisp's path under its crate's directory, after the crate's package name): two rows of this {for} share the key 3"
)]
fn rows_that_share_a_key_are_refused_naming_the_site_and_how_to_read_it() {
    // Given
    let rows = [(3, ()), (4, ()), (3, ())];

    // When
    refuse_duplicate_keys(&rows, "vapors-components/src/list.wisp:5:5");
}
