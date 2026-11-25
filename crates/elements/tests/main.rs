//! The crate's integration tests as one binary (docs/plans/crate-structure.md,
//! section 5, step 1): one link of the crate's closure, not one a file.
//! Written by scripts/one-test-binary.sh. A file under tests/ runs only
//! when this declares it: `every_test_file_is_a_module` refuses one it
//! does not.

mod clip_chain;
mod clip_chain_updates;
mod compiled_mount_order;
mod compiled_sharing;
mod compiled_sheet_once;
mod compiled_winners;
mod honoured_compiled;
mod honoured_declared;
mod paint_finite;
mod rust_sheets_refused;
mod support;

mod a11y_act;
mod a11y_act_focus;
mod a11y_act_reach;
mod a11y_drive;
mod a11y_drive_hover;
mod a11y_drive_json;
mod a11y_find;
mod a11y_hidden_label_text;
mod a11y_mirror;
mod a11y_mirror_focus;
mod a11y_mirror_hoisted;
mod a11y_mirror_hoisted_scroll;
mod a11y_mirror_index;
mod a11y_mirror_index_moves;
mod a11y_mirror_labelledby;
mod a11y_mirror_lane_rows;
mod a11y_mirror_lane_shapes;
mod a11y_mirror_number;
mod a11y_mirror_owner;
mod a11y_mirror_refused;
mod a11y_mirror_refused_states;
mod a11y_mirror_refused_take;
mod a11y_mirror_reparent;
mod a11y_mirror_scaling;
mod a11y_mirror_scroll;
mod a11y_mirror_tree;
mod a11y_mirror_wrapper;
mod a11y_placeholder;
mod a11y_projection;
mod a11y_set_value;
mod a11y_tree;
mod a_href;
mod a_href_link_rule;
mod a_href_modifiers;
mod a_href_path;
mod bound_style;
mod bound_style_parity;
mod bound_style_refused;
mod button_wrap;
mod calc_failures;
mod checkbox;
mod checkbox_layout;
mod child_edits;
mod child_edits_declined;
mod child_edits_for;
mod child_edits_marks;
mod clipping;
mod clipping_refused;
mod compiled_sheets;
mod counter;
mod declared_events;
mod decoration;
mod deep_scroll_rows;
mod dispatcher;
mod dom_agreement;
mod equal_writes_bindings;
mod equal_writes_controls;
mod equal_writes_stale;
mod field_scroll;
mod field_scroll_input;
mod focus;
mod focus_fixup;
mod focus_fixup_composing;
mod focus_fixup_handlers;
mod focus_handlers_move;
mod focus_review;
mod focus_style;
mod focus_window;
mod focus_window_held;
mod frame_clock;
mod global_sheets;
mod grid_off;
mod html_none;
mod html_none_focus;
mod img;
mod img_device_limit;
mod img_paint;
mod img_refused;
mod incremental_inherit;
mod incremental_layout;
mod incremental_random;
mod incremental_scroll;
mod incremental_styles;
mod incremental_text;
mod incremental_text_widths;
mod inline;
mod inline_baseline;
mod inline_hits;
mod inline_metrics;
mod inline_styles;
mod insert_checks_rows;
mod insert_restyle;
mod insert_restyle_rows;
mod kept_batch_keys;
mod kept_batches;
mod kept_box_from;
mod kept_checks;
mod kept_checks_moved;
mod kept_checks_text;
mod kept_clips;
mod kept_containing_blocks;
mod kept_hits;
mod kept_in_place;
mod kept_index_moves;
mod kept_maps;
mod kept_order;
mod kept_paint;
mod kept_placement;
mod kept_shift;
mod kept_shift_checkbox;
mod kept_slabs;
mod key_fields;
mod key_use;
mod keys;
mod lane_batches;
mod lane_block;
mod lane_block_stale;
mod lane_chain;
mod lane_children;
mod lane_drift;
mod lane_flex;
mod lane_flex_heights;
mod lane_frame_declines;
mod lane_frames;
mod lane_frames_moved;
mod lane_ink_spliced;
mod lane_lend;
mod lane_scrollers;
mod lane_shapes;
mod lane_starts;
mod listener_removal;
mod listener_removal_more;
mod listener_removal_review;
mod listener_removal_sweep;
mod marks_oracle_dom;
mod marks_oracle_edits;
mod marks_oracle_owners;
mod marks_restyle;
mod narrow_rebuild;
mod nested_owners;
mod none_refusals;
mod none_refusals_kept;
mod number_host;
mod number_input;
mod number_paint;
mod number_scrub;
mod number_select_all;
mod number_shown;
mod number_shown_follows;
mod number_state;
mod order_splice;
mod order_splice_anchors;
mod order_splice_declined;
mod order_splice_long;
mod order_splice_moved;
mod outline;
mod outline_scale;
mod paint;
mod paint_labels;
mod placeholder;
mod placeholder_number;
mod placeholder_refused;
mod pointer_capture;
mod pointer_capture_edges;
mod pointer_capture_review;
mod pointer_dblclick;
mod pointer_dispatch;
mod pointer_hit;
mod pointer_kind;
mod pointer_states;
mod positioned;
mod positioned_rows;
mod range_keys;
mod range_paint;
mod range_pointer;
mod range_refused;
mod refusals;
mod refusals_button;
mod refusals_declared;
mod rehover_changed;
mod same_value_writes;
mod scene_anchors;
mod scene_appearance;
mod scene_hits;
mod scene_hits_moved;
mod scene_hits_refusals;
mod scene_used;
mod scoped_global;
mod scoped_styles;
mod scroll;
mod scroll_axes;
mod scroll_axes_layout;
mod scroll_chain;
mod scroll_event;
mod scroll_frames;
mod scroll_offsets;
mod scroll_range_kept;
mod scroll_range_overflow;
mod scroll_range_refusals;
mod scrollbar;
mod scrollbar_fade;
mod select_all_platform;
mod shaped_once;
mod sparse_updates;
mod sparse_updates_flex;
mod splice_patched;
mod stacking;
mod style_key_refusals;
mod style_keys;
mod style_sharing;
mod style_sharing_container;
mod style_sharing_parents;
mod style_sharing_presentation;
mod surface_element;
mod surface_element_refused;
mod svg;
mod svg_basic_shape_refusals;
mod svg_basic_shapes;
mod svg_limit;
mod svg_paint;
mod svg_paint_css;
mod svg_paint_updates;
mod svg_path_refused;
mod svg_refused;
mod svg_route;
mod svg_scope;
mod svg_shape_refusals;
mod svg_size;
mod switch;
mod switch_refused;
mod tab_focusability;
mod text_bridge;
mod text_field;
mod text_ime_undo;
mod text_marks_baseline;
mod text_paint;
mod text_paste;
mod text_undo;
mod text_undo_platform;
mod text_words;
mod tooltip_text;
mod touch_pointers;
mod touch_pointers_review;
mod tree_calls;
mod tree_changes;
mod typing_after_hide;
mod ua_focus;
mod update_counts;
mod used_sharing;
mod viewport_signals;
mod window_keys;

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
         and in the file `use crate::<dir>;` in place of a plain `mod <dir>;` of a shared tests/<dir>/, \
         and `use crate::<module> as <name>;` in place of a `#[path]` include of a support file \
         main.rs declares (its #[path] lines name each); reproduce: cargo nextest run -p {} --test \
         tests every_test_file_is_a_module",
        env!("CARGO_PKG_NAME")
    );
}
