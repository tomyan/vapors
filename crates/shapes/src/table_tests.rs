//! The table's lookup and its check: a bad entry refused naming the
//! crate and the site, each shape held once, one format, each hash
//! its bytes' hash.

use super::*;

fn entry(site: &'static str, canonical: &'static [u8]) -> ShapeEntry {
    ShapeEntry {
        site,
        canonical,
        hash: hash_bytes(canonical),
        geometry: Geometry::Ellipse {
            centre: [8.0, 8.0],
            radii: [4.0, 4.0],
        },
    }
}

fn undrawable(site: &'static str) -> ShapeEntry {
    ShapeEntry {
        geometry: Geometry::Rect {
            origin: [0.0; 2],
            size: [0.0, 1.0],
            radius: [0.0; 2],
        },
        ..entry(site, b"Frect")
    }
}

/// A table of `entries`, which live for the test binary's life.
fn table(entries: Vec<ShapeEntry>) -> ShapeTable {
    ShapeTable::new("demo", TABLE_FORMAT, Box::leak(entries.into_boxed_slice()))
}

fn refused(table: &ShapeTable) -> TableError {
    table.check().expect_err("refused")
}

#[test]
fn an_entry_is_found_by_the_index_its_element_names() {
    // Given
    let table = table(vec![
        entry("a.wisp:3", b"FM0 0L1 1"),
        entry("a.wisp:9", b"FM0 0L2 2"),
    ]);

    // When
    let found = table.entry(ShapeIndex::new(1));

    // Then
    assert_eq!(found, Ok(&table.entries()[1]));
    assert_eq!(found.map(|e| e.site), Ok("a.wisp:9"));
    assert_eq!(table.crate_name(), "demo");
}

#[test]
fn an_index_past_the_end_is_refused_naming_the_crate() {
    // When
    let error = table(vec![entry("a.wisp:3", b"FM0 0L1 1")])
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
    // When, Then
    assert_eq!(
        table(vec![
            entry("a.wisp:3", b"FM0 0L1 1"),
            entry("b.wisp:4", b"FM0 0L2 2"),
        ])
        .check(),
        Ok(())
    );
    assert_eq!(table(vec![]).check(), Ok(()));
}

#[test]
fn an_entry_that_cannot_be_drawn_is_refused_naming_crate_and_site() {
    // Given
    let first = table(vec![
        undrawable("b.wisp:7"),
        entry("a.wisp:3", b"FM0 0L1 1"),
    ]);
    let last = table(vec![
        entry("a.wisp:3", b"FM0 0L1 1"),
        undrawable("b.wisp:7"),
    ]);

    // When, Then
    for table in [first, last] {
        let error = refused(&table);
        assert_eq!(error.crate_name, "demo");
        assert_eq!(error.site, Some("b.wisp:7"));
        assert_eq!(error.reason, "size [0.0, 1.0] is not above zero");
        assert_eq!(error.fix, EMITTER);
    }
}

#[test]
fn an_entry_with_no_site_or_no_bytes_is_refused() {
    // When, Then
    assert_eq!(
        refused(&table(vec![entry("", b"FM0 0L1 1")])).reason,
        "the site is empty; an entry names where its shape was written"
    );
    assert_eq!(
        refused(&table(vec![entry("a.wisp:3", b"")])).reason,
        "the canonical bytes are empty; an entry carries its shape's canonical form"
    );
}

#[test]
fn a_hash_that_is_not_its_bytes_hash_is_refused_naming_both() {
    // Given
    let wrong = ShapeEntry {
        hash: 0xabc,
        ..entry("a.wisp:3", b"FM0 0L1 1")
    };

    // When
    let error = refused(&table(vec![wrong]));

    // Then
    assert_eq!(error.site, Some("a.wisp:3"));
    assert_eq!(
        error.reason,
        format!(
            "carries hash 0xabc but its canonical bytes hash to {:#x}",
            hash_bytes(b"FM0 0L1 1")
        )
    );
}

#[test]
fn another_format_or_an_empty_crate_name_is_refused_before_the_entries() {
    // Given
    let entries: &'static [ShapeEntry] = Box::leak(vec![undrawable("b.wisp:7")].into_boxed_slice());

    // When
    let format = ShapeTable::new("demo", TABLE_FORMAT + 1, entries)
        .check()
        .expect_err("refused");
    let name = ShapeTable::new("", TABLE_FORMAT, entries)
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
    // When
    let error = refused(&table(vec![
        entry("a.wisp:3", b"FM0 0L1 1"),
        entry("b.wisp:4", b"FM0 0L2 2"),
        entry("c.wisp:5", b"FM0 0L1 1"),
    ]));

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
    // Given: a collision, which the hash check in front of this path
    // keeps out of `check` unless it is genuine, so the refusal is
    // tested on its own
    let earlier = entry("a.wisp:3", b"FM0 0L1 1");
    let entry = ShapeEntry {
        hash: earlier.hash,
        ..entry("c.wisp:5", b"FM0 0L2 2")
    };

    // When
    let error = table(vec![]).hash_shared(&earlier, &entry);

    // Then
    assert_eq!(
        error.to_string(),
        format!(
            "shape table of crate demo, entry at c.wisp:5: has the hash {:#x} of the entry at a.wisp:3 with different canonical bytes; a hash names one shape; change one shape's canonical bytes (another stroke width, or a point added to the path), or widen the hash",
            earlier.hash
        )
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

#[test]
fn an_entry_whose_bytes_begin_with_no_paint_is_refused() {
    // When, Then: the store reads a closed form's paint from the bytes
    assert_eq!(
        refused(&table(vec![entry("a.wisp:3", b"M0 0L1 1")])).reason,
        "the canonical bytes begin [77, 48, 32, 48, 76], not a paint (F, E, or S and a width)"
    );
}
