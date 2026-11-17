//! `Declared` holds one bit per longhand: the table must fit, and a
//! longhand past the bits is refused naming the fix, never shifted.

use super::*;

#[test]
fn the_property_table_fits_declared_s_bits() {
    // Then
    assert!(
        PROPERTIES.len() <= BITS,
        "the property table has {} longhands, more than Declared's {BITS} bits; fix: widen \
         Declared in crates/css/src/cascade/declared.rs",
        PROPERTIES.len()
    );
}

#[test]
#[should_panic(expected = "has no bit in Declared's 128; fix: widen Declared")]
fn a_longhand_past_the_bits_is_refused_naming_the_fix() {
    // When
    let _ = bit(BITS);
}

#[test]
fn the_last_bit_is_the_top_one() {
    // Then
    assert_eq!(bit(BITS - 1), 1u128 << 127);
}
