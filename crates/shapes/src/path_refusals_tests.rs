//! Path data outside the subset, refused with the offset of the first
//! character not understood.

use super::*;

fn refused(data: &str) -> PathError {
    match IconPath::parse(data) {
        Ok(path) => panic!("{data:?} parsed to {:?}", path.commands()),
        Err(error) => error,
    }
}

#[test]
fn malformed_data_is_refused_at_the_first_character_not_understood() {
    let cases = [
        ("", 0, "empty"),
        ("   ", 3, "empty"),
        ("L1 1", 0, "must start with M"),
        ("M1", 2, "number"),
        ("M1 1L", 5, "number"),
        ("M1 1 L2 x", 8, "number"),
        ("M1 1#", 4, "command"),
        ("M1 1a4 4 0 2 0 1 1", 11, "flag"),
        ("M1 1z2", 5, "command"),
        ("M1..", 3, "number"),
        ("M1e 1", 3, "exponent"),
        ("M1 1L1e39 2", 5, "32-bit"),
        ("M1 1L2 -1e39", 7, "32-bit"),
        ("M,1 1", 1, "comma cannot follow"),
        ("M1 1L ,2 2", 6, "comma cannot follow"),
        ("M1 1é", 4, "'é'"),
        ("M1 1a4 4 0 é", 11, "'é'"),
    ];
    for (data, offset, says) in cases {
        // When
        let error = refused(data);

        // Then
        assert_eq!(error.offset, offset, "{data:?}: {error}");
        assert!(error.to_string().contains(says), "{data:?}: {error}");
    }
}

#[test]
fn every_refusal_points_at_the_doc() {
    for data in [
        "",
        "L1 1",
        "M1",
        "M1 1#",
        "M1 1a4 4 0 2 0 1 1",
        "M1e 1",
        "M1 1L1e39 1",
        "M,1 1",
        "M1 1S1 1 1 1",
    ] {
        // When
        let message = refused(data).to_string();

        // Then
        assert!(message.contains("docs/icons.md"), "{data:?}: {message}");
    }
}

#[test]
fn a_comma_between_argument_groups_is_still_allowed() {
    // When
    let path = IconPath::parse("M1 1L2 2,3 3");

    // Then
    assert!(path.is_ok(), "{path:?}");
}

#[test]
fn smooth_curves_are_refused_naming_the_subset_and_the_fix() {
    // When
    let error = refused("M1 1C2 2 3 3 4 4S5 5 6 6");

    // Then
    assert_eq!(error.offset, 16);
    let message = error.to_string();
    assert!(
        message.contains("'S'")
            && message.contains("M L H V C Q A Z")
            && message.contains("reflected control point")
            && message.contains("docs/icons.md"),
        "{message}"
    );
}
