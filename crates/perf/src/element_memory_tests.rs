use std::collections::BTreeMap;

use super::{ElementMemory, parse_line};

fn measured() -> ElementMemory {
    ElementMemory {
        rows: 1000,
        elements: 4002,
        styles_computed: 5,
        total: 900_000,
        owners: [
            408_000, 100_000, 200_000, 50_000, 1_000, 20_000, 120_000, 1_000,
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

fn phases(pairs: &[(&str, i64)]) -> BTreeMap<String, i64> {
    pairs
        .iter()
        .map(|(name, bytes)| ((*name).to_owned(), *bytes))
        .collect()
}

#[test]
fn the_split_charges_the_update_rest_and_puts_the_other_frame_phases_in_the_rest() {
    // Given: the update's 70 holds style 10, layout 20, paint 30
    let phases = phases(&[
        ("update", 70),
        ("style", 10),
        ("layout", 20),
        ("paint", 30),
        ("accessibility", 5),
        ("draw", 100),
        ("input", 1),
        ("tasks", 40),
    ]);

    // When
    let owners = ElementMemory::split(1000, &phases, 1200);

    // Then: the rest is 1200 - 1000 - (70 + 5 + 100), input and tasks in it
    assert_eq!(owners, [1000, 10, 20, 30, 10, 5, 100, 25]);
}

#[test]
fn the_owners_sum_to_the_total_whatever_the_phases() {
    let cases = [
        phases(&[]),
        phases(&[("tasks", 5_000), ("record", -7)]),
        phases(&[("update", -40), ("style", 9), ("layout", -60), ("paint", 3)]),
        phases(&[("draw", 1 << 40), ("tick", 2), ("input", 3), ("other", 99)]),
    ];
    for (at, phases) in cases.iter().enumerate() {
        let owners = ElementMemory::split(17, phases, 123_456);
        assert_eq!(owners.iter().sum::<i64>(), 123_456, "case {at}: {owners:?}");
    }
}

#[test]
fn a_measurement_whose_owners_miss_the_total_is_refused() {
    // Given
    let mut memory = measured();
    memory.owners[7] -= 1;

    // When
    let refused = memory.checked();

    // Then
    let error = refused.expect_err("one byte short");
    assert!(error.contains("not the total 900000"), "{error}");
}

#[test]
fn an_owner_freeing_more_than_a_percent_of_the_total_is_refused_naming_it() {
    // Given: the update's rest at -2% of the total, the rest making up for it
    let mut memory = measured();
    memory.owners[4] = -18_000;
    memory.owners[7] += 19_000;

    // When
    let refused = memory.checked();

    // Then
    let error = refused.expect_err("below -1%");
    assert!(error.starts_with("update_rest is -18000 bytes"), "{error}");
}

#[test]
fn a_measurement_within_the_limits_is_kept() {
    // Given: a small negative, its owners summing to the total
    let mut memory = measured();
    memory.owners[4] = -1_000;
    memory.owners[7] += 2_000;

    // When
    let kept = memory.clone().checked();

    // Then
    assert_eq!(kept, Ok(memory));
}

#[test]
fn an_owner_at_the_negative_limit_is_kept_and_one_byte_under_it_refused() {
    // Given: the limit is -1% of 900,000, -9,000 bytes; layout frees
    // what the mount left
    let at_limit = |bytes: i64| {
        let mut memory = measured();
        memory.owners[2] = bytes;
        memory.owners[0] = 608_000 - bytes;
        memory
    };

    // When
    let kept = at_limit(-9_000).checked();
    let refused = at_limit(-9_001).checked();

    // Then
    assert!(kept.is_ok(), "{kept:?}");
    assert!(
        refused
            .expect_err("one under")
            .starts_with("layout is -9001 bytes")
    );
}

#[test]
fn a_negative_total_is_refused() {
    // Given
    let mut memory = measured();
    memory.total = -900_000;
    memory.owners = [-900_000, 0, 0, 0, 0, 0, 0, 0];

    // When
    let refused = memory.checked();

    // Then
    assert!(
        refused
            .expect_err("negative")
            .starts_with("the total is -900000")
    );
}

#[test]
fn bytes_no_named_phase_holds_over_a_percent_of_the_total_are_refused() {
    // Given: the rest holds 2% of the total, mount making room
    let mut memory = measured();
    memory.owners[7] = 18_000;
    memory.owners[0] -= 17_000;

    // When
    let refused = memory.checked();

    // Then
    let error = refused.expect_err("rest over 1%");
    assert!(error.starts_with("rest holds 18000 bytes"), "{error}");
}
