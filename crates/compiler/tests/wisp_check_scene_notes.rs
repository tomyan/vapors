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

#[test]
fn a_mesh_a_rule_or_its_style_colours_is_not_noted() {
    // Given: by class, by a directive's class, by tag, by `style`
    let style = "<style>\n.hull { color: red; }\n.lit { color: blue; }\nmesh.wheel { color: black; }\n</style>\n";
    let tags = "<Mesh class=\"hull\" />\n<Mesh class:lit={on} />\n<Mesh class=\"wheel\" />\n<Mesh style=\"color: red\" />";

    // When
    let (output, _) = checked("scene_note_declared", &format!("{style}{tags}"));

    // Then
    assert_eq!(notes(&output), Vec::<String>::new(), "{output:?}");
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
