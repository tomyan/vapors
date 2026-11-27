//! The crate's integration tests as one binary (docs/plans/crate-structure.md,
//! section 5, step 1): one link of the crate's closure, not one a file.
//! Written by scripts/one-test-binary.sh. A file under tests/ runs only
//! when this declares it: `every_test_file_is_a_module` refuses one it
//! does not.

mod codegen_style_only;
mod common;
#[path = "common/build.rs"]
mod common_build;
#[path = "common/codegen.rs"]
mod common_codegen;
#[path = "common/props.rs"]
mod common_props;

mod build_dir;
mod build_dom_path;
mod build_global;
mod build_props;
mod build_props_anchor;
mod build_props_report;
mod build_props_required;
mod build_props_resolve;
mod build_script;
mod build_sites;
mod build_snippet_mount;
mod build_styles;
mod codegen_attribute_refusals;
mod codegen_basic_shape_refusals;
mod codegen_bind;
mod codegen_bind_this;
mod codegen_bind_this_refusals;
mod codegen_bind_this_rows;
mod codegen_blocks;
mod codegen_class_directive;
mod codegen_clones;
mod codegen_component;
mod codegen_component_anchor;
mod codegen_dispatcher;
mod codegen_events;
mod codegen_key_winners;
mod codegen_lints;
mod codegen_names;
mod codegen_page_scope;
mod codegen_read_props;
mod codegen_refusals;
mod codegen_required_props;
mod codegen_snapshot;
mod codegen_snippet_refusals;
mod codegen_snippets;
mod codegen_style_bindings;
mod codegen_style_key;
mod codegen_style_key_limits;
mod codegen_text;
mod codegen_unwrap;
mod codegen_window;
mod every_wisp_keyed;
mod parser_attributes;
mod parser_components;
mod parser_control_flow;
mod parser_elements;
mod parser_entities;
mod parser_for;
mod parser_holes;
mod parser_html;
mod parser_naming;
mod parser_refusals;
mod parser_snippets;
mod parser_text;
mod parser_whitespace;
mod parser_window;
mod sections;
mod styles_analysis;
mod wisp_check;
mod wisp_check_scene_notes;

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
