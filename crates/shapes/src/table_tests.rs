//! The table's lookup and its check: a bad entry refused naming the
//! crate and the site, each shape held once, one format.

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

const fn undrawable(site: &'static str, hash: u64) -> ShapeEntry {
    ShapeEntry {
        site,
        canonical: b"rect",
        hash,
        geometry: Geometry::Rect {
            origin: [0.0; 2],
            size: [0.0, 1.0],
            radius: [0.0; 2],
        },
    }
}

fn table(entries: &'static [ShapeEntry]) -> ShapeTable {
    ShapeTable::new("demo", TABLE_FORMAT, entries)
}

fn refused(table: &ShapeTable) -> TableError {
    table.check().expect_err("refused")
}

#[test]
fn an_entry_is_found_by_the_index_its_element_names() {
    // Given
    static ENTRIES: [ShapeEntry; 2] = [
        entry("a.wisp:3", b"M0 0L1 1", 1),
        entry("a.wisp:9", b"M0 0L2 2", 2),
    ];
    let table = table(&ENTRIES);

    // When
    let found = table.entry(ShapeIndex::new(1));

    // Then
    assert_eq!(found, Ok(&ENTRIES[1]));
    assert_eq!(table.crate_name(), "demo");
    assert_eq!(table.entries().len(), 2);
}

#[test]
fn an_index_past_the_end_is_refused_naming_the_crate() {
    // Given
    static ENTRIES: [ShapeEntry; 1] = [entry("a.wisp:3", b"M0 0L1 1", 1)];

    // When
    let error = table(&ENTRIES)
        .entry(ShapeIndex::new(1))
        .expect_err("past the end");

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

    // When, Then
    assert_eq!(table(&ENTRIES).check(), Ok(()));
    assert_eq!(table(&[]).check(), Ok(()));
}

#[test]
fn an_entry_that_cannot_be_drawn_is_refused_naming_crate_and_site() {
    // Given
    static FIRST: [ShapeEntry; 2] = [undrawable("b.wisp:7", 1), entry("a.wisp:3", b"M0 0L1 1", 2)];
    static LAST: [ShapeEntry; 2] = [entry("a.wisp:3", b"M0 0L1 1", 1), undrawable("b.wisp:7", 2)];

    // When, Then
    for entries in [&FIRST, &LAST] {
        let error = refused(&table(entries));
        assert_eq!(error.crate_name, "demo");
        assert_eq!(error.site, Some("b.wisp:7"));
        assert_eq!(error.reason, "size [0.0, 1.0] is not above zero");
        assert_eq!(error.fix, EMITTER);
    }
}

#[test]
fn an_entry_with_no_site_or_no_bytes_is_refused() {
    // Given
    static NO_SITE: [ShapeEntry; 1] = [entry("", b"M0 0L1 1", 1)];
    static NO_BYTES: [ShapeEntry; 1] = [entry("a.wisp:3", b"", 1)];

    // When, Then
    assert_eq!(
        refused(&table(&NO_SITE)).reason,
        "the site is empty; an entry names where its shape was written"
    );
    assert_eq!(
        refused(&table(&NO_BYTES)).reason,
        "the canonical bytes are empty; an entry carries its shape's canonical form"
    );
}

#[test]
fn another_format_or_an_empty_crate_name_is_refused_before_the_entries() {
    // Given
    static ENTRIES: [ShapeEntry; 1] = [undrawable("b.wisp:7", 1)];

    // When
    let format = ShapeTable::new("demo", TABLE_FORMAT + 1, &ENTRIES)
        .check()
        .expect_err("refused");
    let name = ShapeTable::new("", TABLE_FORMAT, &ENTRIES)
        .check()
        .expect_err("refused");

    // Then
    assert_eq!(
        format.to_string(),
        format!(
            "shape table of crate demo: is in table format {} and this vapors-shapes reads format {TABLE_FORMAT}; build every crate with one vapors-compiler, the one this runtime matches",
            TABLE_FORMAT + 1
        )
    );
    assert_eq!(name.site, None);
    assert_eq!(name.reason, "the crate name is empty");
}

#[test]
fn the_same_shape_twice_is_refused_as_the_emitters_bug() {
    // Given
    static ENTRIES: [ShapeEntry; 3] = [
        entry("a.wisp:3", b"M0 0L1 1", 1),
        entry("b.wisp:4", b"M0 0L2 2", 2),
        entry("c.wisp:5", b"M0 0L1 1", 1),
    ];

    // When
    let error = refused(&table(&ENTRIES));

    // Then
    assert_eq!(error.site, Some("c.wisp:5"));
    assert_eq!(
        error.reason,
        "is the same shape as the entry at a.wisp:3; one crate's table holds each shape once"
    );
    assert_eq!(error.fix, EMITTER);
}

#[test]
fn one_hash_for_two_shapes_is_refused_naming_both_sites() {
    // Given
    static ENTRIES: [ShapeEntry; 2] = [
        entry("a.wisp:3", b"M0 0L1 1", 0xabc),
        entry("c.wisp:5", b"M0 0L2 2", 0xabc),
    ];

    // When
    let error = refused(&table(&ENTRIES));

    // Then
    assert_eq!(
        error.to_string(),
        "shape table of crate demo, entry at c.wisp:5: has the hash 0xabc of the entry at a.wisp:3 with different canonical bytes; a hash names one shape; change one shape's canonical bytes (another stroke width, or a point added to the path), or widen the hash"
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
