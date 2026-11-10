//! Path data outside the icon subset, refused with the offset of the
//! first character not understood.

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
fn curves_are_refused_naming_the_subset() {
    // When
    let error = refused("M1 1C2 2 3 3 4 4");

    // Then
    assert_eq!(error.offset, 4);
    let message = error.to_string();
    assert!(
        message.contains("'C'")
            && message.contains("M L H V A Z")
            && message.contains("docs/icons.md"),
        "{message}"
    );
}
