//! The a11y projection after each step of a script, played on vapors
//! and compared with Chromium's (`ax_projection`): vapors' combobox
//! keyed as APG's own example was (5-A1's start), and one control of
//! each role the a11y driver finds (R1).

use vapors_elements_corpus::events_compare::compare;
use vapors_elements_corpus::events_script::parse;
use vapors_elements_corpus::events_vapors::play;

#[path = "corpus_support/events.rs"]
mod events;

use events::{VIEWPORT, fixture, read};

#[test]
fn the_combobox_script_projects_on_vapors_as_on_apgs_example() {
    // Given: vapors' combobox component, keyed as APG's own example was
    // in Chromium (apg/, see apg/NOTICE.md)
    let script = parse("combobox", &read("combobox.events")).expect("parses");
    let answers = serde_json::from_str(&read("combobox.events.json")).expect("json");

    // When
    let ours = play(&fixture("combobox"), &script, VIEWPORT).unwrap_or_else(|e| panic!("{e}"));

    // Then: the projection, focus, values and selections after each key
    compare("combobox", &script, &ours, &answers).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn the_roles_script_projects_on_vapors_as_on_chromium() {
    // Given: one control of each role the a11y driver finds (R1)
    let script = parse("roles", &read("roles.events")).expect("parses");
    let answers = serde_json::from_str(&read("roles.events.json")).expect("json");

    // When
    let ours = play(&fixture("roles"), &script, VIEWPORT).unwrap_or_else(|e| panic!("{e}"));

    // Then: the roles, names, values and states after each key
    compare("roles", &script, &ours, &answers).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn the_names_script_projects_on_vapors_as_on_chromium() {
    // Given: names from content, labels that fall through, role lists,
    // ARIA range values and nodes left out of the tree (R1)
    let script = parse("names", &read("names.events")).expect("parses");
    let answers = serde_json::from_str(&read("names.events.json")).expect("json");

    // When
    let ours = play(&fixture("names"), &script, VIEWPORT).unwrap_or_else(|e| panic!("{e}"));

    // Then: the names, values and the nodes left out
    compare("names", &script, &ours, &answers).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn the_icon_names_script_projects_on_vapors_as_on_chromium() {
    // Given: decorative and named icons, alone and in buttons, and a
    // named icon that is visibility: hidden (docs/icons.md)
    let script = parse("icon_names", &read("icon_names.events")).expect("parses");
    let answers = serde_json::from_str(&read("icon_names.events.json")).expect("json");

    // When
    let ours = play(&fixture("icon_names"), &script, VIEWPORT).unwrap_or_else(|e| panic!("{e}"));

    // Then: the buttons' names and the images in the tree
    compare("icon_names", &script, &ours, &answers).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn the_titles_script_projects_on_vapors_as_on_chromium() {
    // Given: titles that name a node nothing else names, titles that
    // describe a named node, aria-describedby over a title, and titles
    // of only whitespace (docs/tooltips.md)
    let script = parse("titles", &read("titles.events")).expect("parses");
    let answers = serde_json::from_str(&read("titles.events.json")).expect("json");

    // When
    let ours = play(&fixture("titles"), &script, VIEWPORT).unwrap_or_else(|e| panic!("{e}"));

    // Then: each title as the name or the description Chromium gives
    compare("titles", &script, &ours, &answers).unwrap_or_else(|e| panic!("{e}"));
}
