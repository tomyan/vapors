//! `wisp-check`'s note on a `<Mesh>` whose `color` no rule of its file
//! can declare (docs/plans/css-for-3d.md C3, risk 2): its colour is then
//! whatever the HTML around the scene gives (`body { color }`), which a
//! page may not mean. A note, not a refusal: the exit is 0.

#![cfg(not(target_family = "wasm"))]

#[path = "common/build.rs"]
mod build;

use std::process::Output;

use build::{dirs, write};

/// `wisp-check` on `wisp` written as `page.wisp`; its output, and the
/// file's path as printed.
fn checked(test: &str, wisp: &str) -> (Output, String) {
    let (src, _) = dirs(test);
    write(&src, "page.wisp", wisp);
    let file = src.join("page.wisp");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_wisp-check"))
        .arg(&file)
        .output()
        .expect("run wisp-check");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    (output, file.display().to_string())
}

fn notes(output: &Output) -> Vec<String> {
    (String::from_utf8_lossy(&output.stdout).lines())
        .filter(|line| line.contains(": note: "))
        .map(str::to_owned)
        .collect()
}

#[test]
fn a_mesh_no_rule_colours_is_noted_at_its_tag() {
    // Given: the mesh on line 3, inside a {for}
    let wisp = "<style>\n.hull { color: red; }\n</style>\n{for x in [1] key=x}\n  <Mesh geometry={g} />\n{/for}";

    // When
    let (output, file) = checked("scene_note_unclassed", wisp);

    // Then
    let notes = notes(&output);
    assert_eq!(notes.len(), 1, "{output:?}");
    assert!(
        notes[0].starts_with(&format!("{file}:5:3: note: <Mesh>")) && notes[0].contains("`color`"),
        "{notes:?}"
    );
}

/// The notes for a page of `style` rules and `tags`.
fn notes_for(test: &str, style: &str, tags: &str) -> Vec<String> {
    let (output, _) = checked(test, &format!("<style>\n{style}\n</style>\n{tags}"));
    notes(&output)
}

#[test]
fn a_mesh_coloured_by_its_class_is_not_noted() {
    let notes = notes_for(
        "scene_note_class",
        ".hull { color: red; }",
        "<Mesh class=\"hull\" />",
    );
    assert_eq!(notes, Vec::<String>::new());
}

#[test]
fn a_mesh_coloured_by_a_directives_class_is_not_noted() {
    let notes = notes_for(
        "scene_note_toggle",
        ".lit { color: blue; }",
        "<Mesh class:lit={on} />",
    );
    assert_eq!(notes, Vec::<String>::new());
}

#[test]
fn a_mesh_coloured_by_its_tag_and_class_is_not_noted() {
    let notes = notes_for(
        "scene_note_tag",
        "mesh.wheel { color: black; }",
        "<Mesh class=\"wheel\" />",
    );
    assert_eq!(notes, Vec::<String>::new());
}

#[test]
fn a_mesh_coloured_by_its_literal_style_is_not_noted() {
    let notes = notes_for("scene_note_style", "", "<Mesh style=\"color: red\" />");
    assert_eq!(notes, Vec::<String>::new());
}

#[test]
fn a_mesh_coloured_through_var_is_not_noted() {
    // Given: the `--craft-body` pattern
    let style = ".hull { color: var(--craft-body); }";

    // Then
    let notes = notes_for("scene_note_var", style, "<Mesh class=\"hull\" />");
    assert_eq!(notes, Vec::<String>::new());
}

#[test]
fn a_mesh_given_only_a_custom_property_is_noted() {
    let notes = notes_for(
        "scene_note_custom",
        ".hull { --craft-body: red; }",
        "<Mesh class=\"hull\" />",
    );
    assert_eq!(notes.len(), 1, "{notes:?}");
}

#[test]
fn a_mesh_whose_colour_rule_only_inherits_is_noted() {
    for value in ["inherit", "currentcolor", "unset"] {
        let style = format!(".hull {{ color: {value}; }}");
        let notes = notes_for("scene_note_inherit", &style, "<Mesh class=\"hull\" />");
        assert_eq!(notes.len(), 1, "{value}: {notes:?}");
    }
}

#[test]
fn a_mesh_under_an_element_this_file_colours_is_not_noted() {
    // Given: the HUD around the scene declares the colour here
    let tags = "<div class=\"hud\"><Scene><Group><Mesh /></Group></Scene></div>";

    // Then
    let notes = notes_for("scene_note_ancestor", ".hud { color: white; }", tags);
    assert_eq!(notes, Vec::<String>::new());
}

#[test]
fn meshes_in_an_if_a_snippet_and_a_group_are_noted() {
    // Given
    let tags = "{if on}<Mesh />{/if}\n{snippet row()}<Mesh />{/snippet}\n<Group><Mesh /></Group>";

    // Then
    assert_eq!(notes_for("scene_note_blocks", "", tags).len(), 3);
}

#[test]
fn a_group_or_another_component_is_not_noted() {
    // Given: only a mesh reads `color`
    let wisp = "<Group class=\"craft\" />\n<Card />";

    // When
    let (output, _) = checked("scene_note_group", wisp);

    // Then
    assert_eq!(notes(&output), Vec::<String>::new(), "{output:?}");
}
