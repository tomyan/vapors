use super::Built;

const EARLIER: Built = Built {
    pipelines: 2,
    shader_modules: 3,
};
const LATER: Built = Built {
    pipelines: 5,
    shader_modules: 3,
};

#[test]
fn since_is_what_was_built_between_two_readings() {
    // When
    let built = LATER.since(EARLIER);

    // Then
    let expected = Built {
        pipelines: 3,
        shader_modules: 0,
    };
    assert_eq!(built, expected);
}

#[test]
#[should_panic(expected = "call later.since(earlier)")]
fn since_with_its_readings_swapped_panics_naming_the_fix() {
    let _ = EARLIER.since(LATER);
}
