//! The crate's integration tests as one binary (docs/plans/crate-structure.md,
//! section 5, step 1): one link of the crate's closure, not one a file.
//! Written by scripts/one-test-binary.sh. A file under tests/ runs only
//! when this declares it: `every_test_file_is_a_module` refuses one it
//! does not.

#[path = "support/panels.rs"]
mod support_panels;
#[path = "support/paste_page.rs"]
mod support_paste_page;
#[path = "support/surface.rs"]
mod support_surface;
#[path = "support/tooltip.rs"]
mod support_tooltip;
mod surface_input_page;

mod elements_app_animation;
mod elements_app_drag;
mod elements_app_errors;
mod elements_app_events;
mod elements_app_give;
mod elements_app_give_text;
mod elements_app_keys;
mod elements_app_link;
mod elements_app_paste;
mod elements_app_paste_order;
mod elements_app_shortcuts;
mod elements_app_surface_capture;
mod elements_app_surface_input;
mod elements_app_surface_input_state;
mod elements_app_surfaces;
mod elements_app_text_field;
mod elements_app_touch;
mod gpu_a11y_phase;
mod gpu_a11y_phase_settle;
mod gpu_app_animation;
mod gpu_app_animation_sync;
mod gpu_app_samples;
mod gpu_app_start;
mod gpu_app_start_sizes;
mod gpu_app_surface_states;
mod gpu_app_surfaces;
mod gpu_app_two_panels;
mod gpu_app_two_panels_resize;
mod gpu_clip_entries;
mod gpu_cull;
mod gpu_demos;
mod gpu_draw_over;
mod gpu_fixed_draw_costs;
mod gpu_fling_replay;
mod gpu_focus_fixup;
mod gpu_frame_counts;
mod gpu_frame_counts_kinds;
mod gpu_frame_counts_refused;
mod gpu_frame_counts_surface_work;
mod gpu_frame_times;
mod gpu_frame_timing;
mod gpu_icons;
mod gpu_image_side;
mod gpu_kept_batches;
mod gpu_kept_scroll;
mod gpu_kept_text;
mod gpu_kept_text_pressure;
mod gpu_lane_rows;
mod gpu_lane_rows_clipped;
mod gpu_lane_rows_shapes;
mod gpu_lane_rows_text;
mod gpu_lane_rows_writes;
mod gpu_one_pass;
mod gpu_one_submit;
mod gpu_paint_arena_full;
mod gpu_painter_atlas;
mod gpu_runtime_shapes;
mod gpu_samples;
mod gpu_selectionchange;
mod gpu_shape_meshes;
mod gpu_shapes;
mod gpu_stress;
mod gpu_surface;
mod gpu_surface_cull;
mod gpu_surface_lifecycle;
mod gpu_surface_refusals;
mod gpu_tooltip;
mod gpu_tooltip_app;
mod gpu_touch_fling;
mod tooltip_api;

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
