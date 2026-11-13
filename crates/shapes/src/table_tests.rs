//! The table's lookup and its check: a bad entry refused naming the
//! crate and the site, each shape held once.

use super::*;

const fn entry(site: &'static str, canonical: &'static [u8], hash: u64) -> ShapeEntry {
    ShapeEntry {
        site,
        canonical,
        hash,
        geometry: Geometry::Ellipse {
            centre: [8.0, 8.0],
            radii: [4.0, 4.0],
        },
    }
}

#[test]
fn an_entry_is_found_by_the_index_its_element_names() {
    // Given
    static ENTRIES: [ShapeEntry; 2] = [
        entry("a.wisp:3", b"M0 0L1 1", 1),
        entry("a.wisp:9", b"M0 0L2 2", 2),
    ];
    let table = ShapeTable::new("demo", &ENTRIES);

    // When
    let found = table.entry(1);

    // Then
    assert_eq!(found, Ok(&ENTRIES[1]));
    assert_eq!(table.crate_name(), "demo");
    assert_eq!(table.entries().len(), 2);
}

#[test]
fn an_index_past_the_end_is_refused_naming_the_crate() {
    // Given
    static ENTRIES: [ShapeEntry; 1] = [entry("a.wisp:3", b"M0 0L1 1", 1)];
    let table = ShapeTable::new("demo", &ENTRIES);

    // When
    let error = table.entry(1).expect_err("past the end");

    // Then
    assert_eq!(
        error.to_string(),
        "shape table of crate demo: entry 1 was named but the table holds 1; the table is emitted by vapors-compiler from the crate's templates, so fix the emitter"
    );
}

#[test]
fn a_table_of_distinct_drawable_entries_passes_the_check() {
    // Given
    static ENTRIES: [ShapeEntry; 2] = [
        entry("a.wisp:3", b"M0 0L1 1", 1),
        entry("b.wisp:4", b"M0 0L2 2", 2),
    ];

    // When
    let result = ShapeTable::new("demo", &ENTRIES).check();

    // Then
    assert_eq!(result, Ok(()));
    assert_eq!(ShapeTable::new("empty", &[]).check(), Ok(()));
}

#[test]
fn an_entry_that_cannot_be_drawn_is_refused_naming_crate_and_site() {
    // Given
    static ENTRIES: [ShapeEntry; 2] = [
        entry("a.wisp:3", b"M0 0L1 1", 1),
        ShapeEntry {
            site: "b.wisp:7",
            canonical: b"rect",
            hash: 2,
            geometry: Geometry::Rect {
                origin: [0.0; 2],
                size: [0.0, 1.0],
                radius: [0.0; 2],
            },
        },
    ];

    // When
    let error = ShapeTable::new("demo", &ENTRIES)
        .check()
        .expect_err("refused");

    // Then
    assert_eq!(error.crate_name, "demo");
    assert_eq!(error.site, Some("b.wisp:7"));
    assert_eq!(error.reason, "size [0.0, 1.0] is not above zero");
}

#[test]
fn the_same_shape_twice_is_refused_as_the_emitters_bug() {
    // Given
    static ENTRIES: [ShapeEntry; 2] = [
        entry("a.wisp:3", b"M0 0L1 1", 1),
        entry("c.wisp:5", b"M0 0L1 1", 1),
    ];

    // When
    let error = ShapeTable::new("demo", &ENTRIES)
        .check()
        .expect_err("refused");

    // Then
    assert_eq!(error.site, Some("c.wisp:5"));
    assert_eq!(
        error.reason,
        "is the same shape as the entry at a.wisp:3; one crate's table holds each shape once"
    );
}

#[test]
fn one_hash_for_two_shapes_is_refused_naming_both_sites() {
    // Given
    static ENTRIES: [ShapeEntry; 2] = [
        entry("a.wisp:3", b"M0 0L1 1", 0xabc),
        entry("c.wisp:5", b"M0 0L2 2", 0xabc),
    ];

    // When
    let error = ShapeTable::new("demo", &ENTRIES)
        .check()
        .expect_err("refused");

    // Then
    assert_eq!(
        error.to_string(),
        "shape table of crate demo, entry at c.wisp:5: has the hash 0xabc of the entry at a.wisp:3 with different canonical bytes; a hash names one shape; the table is emitted by vapors-compiler from the crate's templates, so fix the emitter"
    );
}

#[test]
fn a_shape_id_is_its_registry_index() {
    // Given
    let id = ShapeId::new(7);

    // Then
    assert_eq!(id.index(), 7);
    assert!(ShapeId::new(3) < id);
}
