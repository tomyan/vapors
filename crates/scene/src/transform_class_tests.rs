use super::TransformClass;

#[test]
fn only_a_whole_translation_snaps() {
    // Given
    let classes = [
        TransformClass::WholeTranslate,
        TransformClass::Affine,
        TransformClass::Projective,
    ];

    // When
    let snaps = classes.map(TransformClass::snaps);

    // Then
    assert_eq!(snaps, [true, false, false]);
}
