//! The `.events` player in a live headless Chromium: a page with a
//! button, a field and a checkbox, driven by a script, its answers read
//! back, and an expectation that does not hold refused. In the gpu tier,
//! with vapors' other Chromium tests.

use std::time::Duration;

use serde_json::json;
use vapors_devtools::Session;
use vapors_elements_corpus::events_play::play;
use vapors_elements_corpus::events_script::parse;
use vapors_text::comparison::Chromium;

/// vapors' `COLD_START`, 145 s, about 2.5x the slowest first launch on a
/// runner (57.9 s); a warm browser answers in milliseconds.
const DEADLINE: Duration = Duration::from_secs(145);

const PAGE: &str = "data:text/html,<body style=margin:0>\
<button id=edit style='position:absolute;left:0;top:0;width:100px;height:40px'>Edit</button>\
<input id=title style='position:absolute;left:0;top:60px;width:200px;height:30px'>\
<div><input type=checkbox style='position:absolute;left:0;top:120px'></div></body>";

const SCRIPT: &str = "\
listen click focusin keydown input pointerdown
click 50 20
expect event click button#edit
expect event pointerdown button#edit
expect focus button#edit
key Tab
expect focus-visible input#title
type \"Wa\"
expect event input input#title
expect no event click
";

fn session(test: &str) -> (vapors_text::comparison::Live, Session) {
    let chromium = Chromium::find().unwrap_or_else(|e| panic!("{e}"));
    let profile = std::env::temp_dir().join(format!("events-{test}-{}", std::process::id()));
    let mut live = chromium
        .live(&profile, 400, 300, 1)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut session =
        Session::attach(&profile, DEADLINE, &mut || live.alive()).unwrap_or_else(|e| panic!("{e}"));
    session
        .call("Page.enable", &json!({}), DEADLINE)
        .unwrap_or_else(|e| panic!("{e}"));
    session
        .call("Page.navigate", &json!({ "url": PAGE }), DEADLINE)
        .unwrap_or_else(|e| panic!("{e}"));
    session
        .wait_event("Page.loadEventFired", DEADLINE)
        .unwrap_or_else(|e| panic!("{e}"));
    (live, session)
}

#[test]
fn gpu_events_a_script_is_played_and_its_answers_recorded() {
    // Given
    let (_live, mut session) = session("plays");
    let script = parse("page", SCRIPT).expect("parses");

    // When
    let answers = play(&mut session, "page", &script, DEADLINE).unwrap_or_else(|e| panic!("{e}"));

    // Then: the click's log, with the pointer's type, and the field's input
    let click = &answers["steps"][0];
    assert_eq!(click["focus"], "button#edit", "{answers:#}");
    let pointer = click["events"]
        .as_array()
        .expect("events")
        .iter()
        .find(|e| e["type"] == "pointerdown")
        .cloned();
    assert_eq!(
        pointer.map(|p| p["pointerType"].clone()),
        Some(json!("mouse")),
        "{answers:#}"
    );
    assert_eq!(answers["steps"][1]["focusVisible"], true, "{answers:#}");
    assert_eq!(
        answers["steps"][2]["events"].as_array().map(Vec::len),
        Some(4),
        "keydown and input per letter: {answers:#}"
    );
}

#[test]
fn gpu_events_an_expectation_that_does_not_hold_is_refused_naming_its_line() {
    // Given: focus goes to the button, but the script says the field; the
    // checkbox, with no id, is named by its path and does take focus
    let (_live, mut session) = session("refuses");
    let script = parse("page", "listen click\nclick 50 20\nexpect focus input#title\nclick 5 125\nexpect focus body>div:nth-child(3)>input:nth-child(1)\n").expect("parses");

    // When
    let refusal = play(&mut session, "page", &script, DEADLINE).expect_err("refused");

    // Then
    assert!(
        refusal.contains("page.events:3: focus is on button#edit, not input#title"),
        "{refusal}"
    );
    assert!(
        !refusal.contains("page.events:5:"),
        "the checkbox's path names it: {refusal}"
    );
}
