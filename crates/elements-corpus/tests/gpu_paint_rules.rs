//! GPU tier: the rules in `paint_check` are themselves checked.
//! Chromium's own rendering with a fixture changed the way a vapors bug
//! would change it (a radius, a shadow, a border width, a 1 px shift)
//! must fail them against the fixture as it is, at 1x and at 2x. One test per change and scale, so
//! each runs two screenshots and stays well inside the test deadline.

mod paint_support;

use paint_support::{chromiums, image, laid_out, paint_fixtures, regions};
use vapors_elements_corpus::Fixture;
use vapors_elements_corpus::paint_check::compare;

fn fixture(name: &str) -> Fixture {
    paint_fixtures()
        .into_iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no paint fixture {name}"))
}

/// Chromium's own rendering with `changed` appended to `name`'s style
/// must fail the rules against the fixture as it is, at `scale`.
fn must_fail(test: &str, name: &str, changed: &str, scale: u32) {
    let theirs = chromiums(test, name, scale, "");
    let mutant = chromiums(test, name, scale, changed);
    let doc = laid_out(&fixture(name), scale);
    let found = compare(
        &image(&mutant),
        &image(&theirs),
        &regions(&doc.borrow(), scale),
    );
    assert!(
        !found.is_empty(),
        "the paint rules let {name} with `{changed}` at {scale}x through"
    );
}

macro_rules! must_fail {
    ($($test:ident: $name:literal, $changed:literal, $scale:literal;)*) => {
        $(
            #[test]
            fn $test() {
                must_fail(stringify!($test), $name, $changed, $scale);
            }
        )*
    };
}

must_fail! {
    gpu_paint_rules_catch_the_card_radius_at_4_at_1x: "paint_boxes", ".card { border-radius: 4px; }", 1;
    gpu_paint_rules_catch_the_card_radius_at_4_at_2x: "paint_boxes", ".card { border-radius: 4px; }", 2;
    gpu_paint_rules_catch_the_card_radius_at_6_at_1x: "paint_boxes", ".card { border-radius: 6px; }", 1;
    gpu_paint_rules_catch_the_card_radius_at_6_at_2x: "paint_boxes", ".card { border-radius: 6px; }", 2;
    gpu_paint_rules_catch_the_card_moved_1px_at_1x: "paint_boxes", ".card { margin-left: 1px; }", 1;
    gpu_paint_rules_catch_the_card_moved_1px_at_2x: "paint_boxes", ".card { margin-left: 1px; }", 2;
    gpu_paint_rules_catch_the_unshadowed_radius_at_6_at_1x: "paint_boxes", ".soft { border-radius: 6px; }", 1;
    gpu_paint_rules_catch_the_unshadowed_radius_at_6_at_2x: "paint_boxes", ".soft { border-radius: 6px; }", 2;
    gpu_paint_rules_catch_a_lone_corner_at_30_at_1x: "paint_boxes", ".corner { border-top-left-radius: 30px; }", 1;
    gpu_paint_rules_catch_a_lone_corner_at_30_at_2x: "paint_boxes", ".corner { border-top-left-radius: 30px; }", 2;
    gpu_paint_rules_catch_unscaled_mixed_radii_at_1x: "paint_boxes", ".mixed { border-radius: 20px; }", 1;
    gpu_paint_rules_catch_unscaled_mixed_radii_at_2x: "paint_boxes", ".mixed { border-radius: 20px; }", 2;
    gpu_paint_rules_catch_a_shadow_blur_at_8_at_1x: "paint_boxes", ".card { box-shadow: 0 4px 8px #00000059; }", 1;
    gpu_paint_rules_catch_a_shadow_blur_at_8_at_2x: "paint_boxes", ".card { box-shadow: 0 4px 8px #00000059; }", 2;
    gpu_paint_rules_catch_a_shadow_offset_at_2_at_1x: "paint_boxes", ".card { box-shadow: 0 2px 12px #00000059; }", 1;
    gpu_paint_rules_catch_a_shadow_offset_at_2_at_2x: "paint_boxes", ".card { box-shadow: 0 2px 12px #00000059; }", 2;
    gpu_paint_rules_catch_a_spread_at_3_at_1x: "paint_boxes", ".spread { box-shadow: 0 0 0 3px #3a7bd5; }", 1;
    gpu_paint_rules_catch_a_spread_at_3_at_2x: "paint_boxes", ".spread { box-shadow: 0 0 0 3px #3a7bd5; }", 2;
    gpu_paint_rules_catch_one_border_side_at_7_at_1x: "paint_boxes", ".sides { border-left-width: 7px; width: 81px; }", 1;
    gpu_paint_rules_catch_one_border_side_at_7_at_2x: "paint_boxes", ".sides { border-left-width: 7px; width: 81px; }", 2;
}
