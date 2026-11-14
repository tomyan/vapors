//! Equal shapes one hash; a changed paint parameter another; a bad
//! stroke width refused naming the shape.

use super::*;

fn canonical(data: &str, paint: Paint) -> Canonical {
    let path = IconPath::parse(data).unwrap_or_else(|e| panic!("{data:?}: {e}"));
    Canonical::of("a.wisp:3", &path, paint).unwrap_or_else(|e| panic!("{data:?}: {e}"))
}

const STROKE: Paint = Paint::Stroke { width: 1.5 };

#[test]
fn the_same_shape_written_two_ways_has_one_canonical_form_and_hash() {
    // Given: relative and absolute forms, implicit repeats, negative zero
    let a = canonical("M1 1l1 0 0 1h-1z", STROKE);
    let b = canonical("M1 1L2 1L2 2L1 2Z", STROKE);
    let zero = canonical("M-0 0L1 1", STROKE);
    let plain = canonical("M0 0L1 1", STROKE);

    // Then
    assert_eq!(a, b);
    assert_eq!(a.hash(), b.hash());
    assert_eq!(zero.bytes(), plain.bytes());
}

#[test]
fn a_changed_stroke_width_fill_rule_or_point_is_another_shape() {
    // Given
    let base = canonical("M0 0L4 0L4 4Z", STROKE);
    let others = [
        canonical("M0 0L4 0L4 4Z", Paint::Stroke { width: 2.0 }),
        canonical("M0 0L4 0L4 4Z", Paint::Fill(FillRule::NonZero)),
        canonical("M0 0L4 0L4 4Z", Paint::Fill(FillRule::EvenOdd)),
        canonical("M0 0L4 0L4 4.5Z", STROKE),
        canonical("M0 0L4 0L4 4", STROKE),
    ];

    // Then
    for other in &others {
        assert_ne!(base.bytes(), other.bytes());
        assert_ne!(base.hash(), other.hash(), "{other:?}");
    }
    assert_ne!(others[1].hash(), others[2].hash());
}

/// `numbers` as little-endian f32 bits.
fn bits(numbers: &[f32]) -> Vec<u8> {
    numbers.iter().flat_map(|n| n.to_le_bytes()).collect()
}

#[test]
fn the_bytes_start_with_the_paint() {
    // When
    let stroke = canonical("M0 0", STROKE);
    let non_zero = canonical("M0 0", Paint::Fill(FillRule::NonZero));
    let even_odd = canonical("M0 0", Paint::Fill(FillRule::EvenOdd));

    // Then
    assert_eq!(&stroke.bytes()[..5], [&b"S"[..], &bits(&[1.5])].concat());
    assert_eq!(non_zero.bytes()[0], b'F');
    assert_eq!(even_odd.bytes()[0], b'E');
    assert_eq!(non_zero.bytes().len(), 1 + 1 + 8);
}

#[test]
fn each_command_is_its_verb_byte_then_its_numbers() {
    // When
    let stroke = canonical("M1 2C3 4 5 6 7 8Q9 10 11 12L13 14Z", STROKE);

    // Then
    let expected = [
        &b"S"[..],
        &bits(&[1.5]),
        b"M",
        &bits(&[1.0, 2.0]),
        b"C",
        &bits(&[3.0, 4.0, 5.0, 6.0, 7.0, 8.0]),
        b"Q",
        &bits(&[9.0, 10.0, 11.0, 12.0]),
        b"L",
        &bits(&[13.0, 14.0]),
        b"Z",
    ]
    .concat();
    assert_eq!(stroke.bytes(), expected);
}

#[test]
fn an_arc_is_its_radii_rotation_flags_and_end() {
    // When: radii large enough for the chord, so the pen leaves them
    let arc = canonical("M0 0A8 9 30 1 0 4 4", Paint::Fill(FillRule::NonZero));

    // Then: the flags share one byte, large in bit 0 and sweep in bit 1
    let expected = [
        &b"FM"[..],
        &bits(&[0.0, 0.0]),
        b"A",
        &bits(&[8.0, 9.0, 30.0]),
        b"\x01>",
        &bits(&[4.0, 4.0]),
    ]
    .concat();
    assert_eq!(arc.bytes(), expected);
}

#[test]
fn the_form_folds_the_parsers_rewrites_and_nothing_beyond_f32_arithmetic() {
    // Then: 0.3 + 0.6 is not 0.9 in f32, so these are two entries (a
    // cost, never a wrong draw); the module doc says so
    assert_ne!(
        canonical("M0.3 0.1l0.6 0", STROKE),
        canonical("M0.3 0.1L0.9 0.1", STROKE)
    );
}

#[test]
fn the_hash_is_fnv1a_64_so_build_and_run_agree_on_every_target() {
    // Then: the published test vectors
    assert_eq!(hash_bytes(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(hash_bytes(b"a"), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(hash_bytes(b"foobar"), 0x8594_4171_f739_67e8);
}

#[test]
fn a_stroke_width_that_is_not_above_zero_is_refused_naming_the_shape() {
    // Given
    let path = IconPath::parse("M0 0L1 1").expect("parses");

    // When, Then
    for width in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let error = Canonical::of("b.wisp:7", &path, Paint::Stroke { width }).expect_err("refused");
        assert_eq!(
            error,
            format!(
                "the shape at b.wisp:7 has stroke-width {width}, which is not a finite number above zero; write a positive width"
            )
        );
    }
}
