use std::collections::BTreeMap;

use super::{ElementMemory, parse_line};

fn measured() -> ElementMemory {
    ElementMemory {
        rows: 1000,
        elements: 4002,
        styles_computed: 5,
        total: 900_000,
        owners: [
            400_000, 100_000, 200_000, 50_000, 1_000, 20_000, 120_000, 9_000,
        ],
    }
}

#[test]
fn a_measurement_survives_its_line() {
    // Given
    let memory = measured();

    // When
    let parsed = parse_line(&memory.line());

    // Then
    assert_eq!(parsed, Ok(memory));
}

#[test]
fn a_line_with_negative_owners_survives_too() {
    // Given
    let mut memory = measured();
    memory.owners[6] = -4096;

    // When
    let parsed = parse_line(&memory.line());

    // Then
    assert_eq!(parsed, Ok(memory));
}

#[test]
fn lines_of_another_shape_are_refused_naming_the_line() {
    let line = measured().line();
    let short = line.rsplit_once(' ').expect("fields").0.to_owned();
    let renamed = line.replace("layout=", "layouts=");
    for bad in [
        String::new(),
        "mount-memory rows=1".into(),
        short,
        renamed,
        format!("{line} extra=1"),
    ] {
        let refused = parse_line(&bad).expect_err("not the shape");
        assert!(refused.contains(&format!("{bad:?}")), "{refused}");
    }
}

#[test]
fn the_split_charges_the_update_rest_and_the_frames_rest() {
    // Given: the update's 70 holds style 10, layout 20, paint 30
    let phases: BTreeMap<String, i64> = [
        ("update", 70),
        ("style", 10),
        ("layout", 20),
        ("paint", 30),
        ("accessibility", 5),
        ("draw", 100),
        ("input", 1),
    ]
    .into_iter()
    .map(|(name, bytes)| (name.to_owned(), bytes))
    .collect();

    // When
    let owners = ElementMemory::split(1000, &phases, 1200);

    // Then: rest is 1200 - 1000 - (70 + 5 + 100 + 1)
    assert_eq!(owners, [1000, 10, 20, 30, 10, 5, 100, 24]);
}
