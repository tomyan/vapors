//! The crate's integration tests as one binary (docs/plans/crate-structure.md,
//! section 5, step 1): one link of the crate's closure, not one a file.
//! Written by scripts/one-test-binary.sh. A file under tests/ runs only
//! when this declares it: `every_test_file_is_a_module` refuses one it
//! does not.

mod support;
#[path = "support/classify.rs"]
mod support_classify;
#[path = "support/winners.rs"]
mod support_winners;

mod analysis_classify;
mod analysis_classify_scoped;
mod analysis_oracle;
mod analysis_winners;
mod analysis_winners_oracle;
mod analysis_winners_presentation;
mod analysis_winners_values;
mod animation_interpolate;
mod animation_keyframes;
mod animation_sampler;
mod animation_spec;
mod animation_timing;
mod animation_timing_curve;
mod animation_transition;
mod cascade;
mod cascade_buckets;
mod cascade_collect;
mod cascade_compute;
mod cascade_compute_resolve;
mod cascade_compute_sources;
mod cascade_compute_var;
mod cascade_conditions;
mod cascade_font_weight;
mod cascade_inline;
mod cascade_media;
mod cascade_media_feature;
mod cascade_media_range;
mod cascade_presentation;
mod cascade_restyle;
mod cascade_sheets;
mod cascade_supports;
mod parse_at_rules;
mod parse_declarations;
mod parse_keyframes;
mod parse_rules;
mod parse_rules_refusals;
mod parse_stream;
mod property_declaration;
mod property_ids;
mod property_names;
mod property_shorthand;
mod property_shorthand_border;
mod property_shorthand_layout;
mod property_shorthand_motion;
mod property_table;
mod selector_attribute;
mod selector_global;
mod selector_matching;
mod selector_nth;
mod selector_parse;
mod selector_pseudo;
mod selector_specificity;
mod selector_structural;
mod serialize;
mod share_inputs;
mod share_selected;
mod syntax_special;
mod syntax_special_grid;
mod syntax_types;
mod token;
mod token_consume;
mod token_cursor;
mod user_agent;
mod value_calc_eval;
mod value_calc_parse;
mod value_color;
mod value_color_parse;
mod value_length;
mod value_var;

/// Every `.rs` file directly under tests/ is a module above, so none is
/// silently left out of the binary.
#[test]
fn every_test_file_is_a_module() {
    let tests = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let main = std::fs::read_to_string(tests.join("main.rs")).expect("tests/main.rs reads");
    let mut missing = Vec::new();
    for entry in std::fs::read_dir(&tests).expect("tests/ lists") {
        let name = entry
            .expect("an entry")
            .file_name()
            .to_string_lossy()
            .into_owned();
        let Some(module) = name.strip_suffix(".rs") else {
            continue;
        };
        if module != "main" && !main.lines().any(|line| line == format!("mod {module};")) {
            missing.push(name);
        }
    }
    assert!(
        missing.is_empty(),
        "tests/{missing:?} not declared in tests/main.rs, so never run; fix: add `mod <name>;` there, \
         and in the file `use crate::<module>;` for each shared module it declared (see \
         scripts/one-test-binary.sh); reproduce: cargo test -p {} --test tests every_test_file_is_a_module",
        env!("CARGO_PKG_NAME")
    );
}
