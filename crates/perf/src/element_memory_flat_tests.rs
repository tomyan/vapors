use super::{SIZES, table, violations};
use crate::element_memory::ElementMemory;

/// A page of `rows` rows (four elements each) whose owners hold `bytes`
/// of its element count, mount first; the total is their sum.
fn page(rows: usize, bytes: impl Fn(i64) -> [i64; 8]) -> ElementMemory {
    let elements = rows * 4 + 2;
    let owners = bytes(i64::try_from(elements).expect("fits"));
    ElementMemory {
        rows,
        elements,
        styles_computed: 4,
        total: owners.iter().sum(),
        owners,
    }
}

/// 2 KB of mount, 7 KB of layout and 2 KB of paint an element.
fn flat(elements: i64) -> [i64; 8] {
    [
        2000 * elements,
        100,
        7000 * elements,
        2000 * elements,
        0,
        0,
        5_600_000,
        0,
    ]
}

fn pages(bytes: impl Fn(i64) -> [i64; 8] + Copy) -> Vec<ElementMemory> {
    SIZES.iter().map(|&rows| page(rows, bytes)).collect()
}

#[test]
fn a_page_whose_bytes_grow_with_its_elements_alone_is_flat() {
    assert_eq!(violations(&pages(flat)), Ok(vec![]));
}

#[test]
fn an_owner_holding_bytes_by_the_rows_per_element_is_named() {
    // Given: paint holds a slot per element for every element, as #327 fixed
    let quadratic = |elements: i64| {
        let mut owners = flat(elements);
        owners[3] += elements * elements / 8;
        owners
    };

    // When
    let found = violations(&pages(quadratic)).expect("the sizes");

    // Then
    assert!(
        found.iter().any(|line| line.starts_with("total:")),
        "{found:?}"
    );
    assert!(
        found.iter().any(|line| line.starts_with("paint:")),
        "{found:?}"
    );
}

#[test]
fn a_small_owner_growing_by_under_the_floor_is_not_named() {
    // Given: style's slope grows 36%, under 512 bytes an element
    let style = |elements: i64| {
        let mut owners = flat(elements);
        owners[1] = 90 * elements + elements * elements / 4000;
        owners
    };

    // When
    let found = violations(&pages(style));

    // Then
    assert_eq!(found, Ok(vec![]));
}

#[test]
fn pages_of_other_sizes_are_refused_naming_them() {
    // Given
    let wrong: Vec<ElementMemory> = [1024, 4096].iter().map(|&rows| page(rows, flat)).collect();

    // When
    let refused = violations(&wrong).expect_err("two sizes");

    // Then
    assert!(refused.contains("[1024, 4096]"), "{refused}");
}

#[test]
fn the_table_shows_each_page_s_bytes_per_element_from_the_page_before() {
    // Given
    let measured = pages(flat);

    // When
    let shown = table(&measured);

    // Then: 4,096 rows add 12,288 elements of 11,000 bytes each
    let line = shown
        .lines()
        .find(|line| line.starts_with("| 4096 |"))
        .expect("a row");
    assert!(line.contains("| 11000 |"), "{line}");
}

#[test]
fn a_style_computed_per_element_is_named() {
    // Given: the largest page computes a style for each of its rows
    let mut measured = pages(flat);
    measured[2].styles_computed = SIZES[2];

    // When
    let found = violations(&measured).expect("the sizes");

    // Then
    assert!(
        found
            .iter()
            .any(|line| line.starts_with("styles computed [4, 4, 16384]")),
        "{found:?}"
    );
}

#[test]
fn a_total_over_twice_layout_per_element_is_named() {
    // Given: 9 KB of paint an element beside 7 KB of layout, flat
    let heavy = |elements: i64| {
        let mut owners = flat(elements);
        owners[3] = 9000 * elements;
        owners
    };

    // When
    let found = violations(&pages(heavy)).expect("the sizes");

    // Then
    assert!(
        found
            .iter()
            .any(|line| line.starts_with("total: 18000 bytes per element, over 2 times")),
        "{found:?}"
    );
}
