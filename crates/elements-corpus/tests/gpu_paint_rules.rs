//! GPU tier: the rules in `paint_check` are themselves checked.
//! Chromium's own rendering of a fixture in each changed state
//! (`paint_reference::STATES`: a radius, a shadow, a border width, a
//! 1 px shift, as a vapors bug would change it) must fail them against
//! its rendering of the fixture as it is, at 1x and at 2x. Both sides
//! are committed saved references; no browser starts. One test per
//! state and scale.

mod paint_support;

use paint_support::{image, laid_out, paint_fixtures, regions, theirs};
use vapors_elements_corpus::Fixture;
use vapors_elements_corpus::paint_check::compare;
use vapors_elements_corpus::paint_reference::{SCALES, STATES};

fn fixture(name: &str) -> Fixture {
    paint_fixtures()
        .into_iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no paint fixture {name}"))
}

/// Chromium's rendering of `state` must fail the rules against its
/// fixture as it is, at `scale`.
fn must_fail(state: &str, scale: u32) {
    let changed = STATES
        .iter()
        .find(|s| s.name == state)
        .unwrap_or_else(|| panic!("no state {state} in paint_reference::STATES"));
    let theirs_as_is = theirs(changed.fixture, None, scale);
    let mutant = theirs(changed.fixture, Some(state), scale);
    let doc = laid_out(&fixture(changed.fixture), scale);
    let found = compare(
        &image(&mutant),
        &image(&theirs_as_is),
        &regions(&doc.borrow(), scale),
    );
    assert!(
        !found.is_empty(),
        "the paint rules let {} with `{}` at {scale}x through",
        changed.fixture,
        changed.css
    );
}

macro_rules! must_fail {
    ($($test:ident: $state:literal, $scale:literal;)*) => {
        const TESTED: &[(&str, u32)] = &[$(($state, $scale)),*];
        $(
            #[test]
            fn $test() {
                must_fail($state, $scale);
            }
        )*
    };
}

#[test]
fn gpu_paint_rules_every_state_is_tested_at_every_scale() {
    let missing: Vec<_> = STATES
        .iter()
        .flat_map(|s| SCALES.map(|scale| (s.name, scale)))
        .filter(|tested| !TESTED.contains(tested))
        .collect();
    assert!(
        missing.is_empty(),
        "states with no test: {missing:?}; add them to `must_fail!` in gpu_paint_rules.rs"
    );
}

must_fail! {
    gpu_paint_rules_catch_the_card_radius_at_4_at_1x: "card_radius_4", 1;
    gpu_paint_rules_catch_the_card_radius_at_4_at_2x: "card_radius_4", 2;
    gpu_paint_rules_catch_the_card_radius_at_6_at_1x: "card_radius_6", 1;
    gpu_paint_rules_catch_the_card_radius_at_6_at_2x: "card_radius_6", 2;
    gpu_paint_rules_catch_the_card_moved_1px_at_1x: "card_moved_1px", 1;
    gpu_paint_rules_catch_the_card_moved_1px_at_2x: "card_moved_1px", 2;
    gpu_paint_rules_catch_the_unshadowed_radius_at_6_at_1x: "unshadowed_radius_6", 1;
    gpu_paint_rules_catch_the_unshadowed_radius_at_6_at_2x: "unshadowed_radius_6", 2;
    gpu_paint_rules_catch_a_lone_corner_at_30_at_1x: "lone_corner_30", 1;
    gpu_paint_rules_catch_a_lone_corner_at_30_at_2x: "lone_corner_30", 2;
    gpu_paint_rules_catch_unscaled_mixed_radii_at_1x: "unscaled_mixed_radii", 1;
    gpu_paint_rules_catch_unscaled_mixed_radii_at_2x: "unscaled_mixed_radii", 2;
    gpu_paint_rules_catch_a_shadow_blur_at_8_at_1x: "shadow_blur_8", 1;
    gpu_paint_rules_catch_a_shadow_blur_at_8_at_2x: "shadow_blur_8", 2;
    gpu_paint_rules_catch_a_shadow_offset_at_2_at_1x: "shadow_offset_2", 1;
    gpu_paint_rules_catch_a_shadow_offset_at_2_at_2x: "shadow_offset_2", 2;
    gpu_paint_rules_catch_a_spread_at_3_at_1x: "spread_3", 1;
    gpu_paint_rules_catch_a_spread_at_3_at_2x: "spread_3", 2;
    gpu_paint_rules_catch_one_border_side_at_7_at_1x: "one_border_side_7", 1;
    gpu_paint_rules_catch_one_border_side_at_7_at_2x: "one_border_side_7", 2;
    gpu_paint_rules_catch_mute_joins_bevelled_at_1x: "mute_joins_bevelled", 1;
    gpu_paint_rules_catch_mute_joins_bevelled_at_2x: "mute_joins_bevelled", 2;
    gpu_paint_rules_catch_an_icon_moved_1px_at_1x: "icon_moved_1px", 1;
    gpu_paint_rules_catch_an_icon_moved_1px_at_2x: "icon_moved_1px", 2;
    gpu_paint_rules_catch_an_icon_scaled_to_18px_at_1x: "icon_scaled_to_18px", 1;
    gpu_paint_rules_catch_an_icon_scaled_to_18px_at_2x: "icon_scaled_to_18px", 2;
    gpu_paint_rules_catch_a_doubled_stroke_width_at_1x: "doubled_stroke_width", 1;
    gpu_paint_rules_catch_a_doubled_stroke_width_at_2x: "doubled_stroke_width", 2;
}
