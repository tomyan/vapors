//! A live headless Chromium driven over the `DevTools` protocol: the
//! accessibility tree, a click and a key through `Input`, and a frame.
//! In the gpu tier, with vapors' other Chromium tests.

use std::time::Duration;

use serde_json::{Value, json};
use vapors_devtools::Session;
use vapors_text::comparison::Chromium;

/// Each call: a cold first launch on a runner took up to 58 s (vapors'
/// `COLD_START`); calls on a warm browser take milliseconds.
const DEADLINE: Duration = Duration::from_secs(90);

const PAGE: &str = "data:text/html,<button id=b onclick=\"document.title='clicked'\">Save</button>\
<input id=i onkeydown=\"document.title='key '+event.key\">";

fn session(test: &str) -> (vapors_text::comparison::Live, Session) {
    let chromium = Chromium::find().unwrap_or_else(|e| panic!("{e}"));
    let profile = std::env::temp_dir().join(format!("devtools-{test}-{}", std::process::id()));
    let live = chromium
        .live(&profile, 400, 300, 1)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut session = Session::attach(live.profile(), DEADLINE).unwrap_or_else(|e| panic!("{e}"));
    call(&mut session, "Page.enable", &json!({}));
    call(&mut session, "Page.navigate", &json!({ "url": PAGE }));
    session
        .wait_event("Page.loadEventFired", DEADLINE)
        .unwrap_or_else(|e| panic!("{e}"));
    (live, session)
}

fn call(session: &mut Session, method: &str, params: &Value) -> Value {
    session
        .call(method, params, DEADLINE)
        .unwrap_or_else(|e| panic!("{e}"))
}

fn title(session: &mut Session) -> String {
    let title = call(
        session,
        "Runtime.evaluate",
        &json!({ "expression": "document.title", "returnByValue": true }),
    );
    title["result"]["value"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn gpu_devtools_reads_the_accessibility_tree_and_drives_input_and_frames() {
    // Given
    let (_live, mut session) = session("all");

    // When: the tree
    let tree = call(&mut session, "Accessibility.getFullAXTree", &json!({}));

    // Then: the button is there by role and name
    let button = tree["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|n| n["role"]["value"] == "button");
    assert_eq!(
        button.map(|b| b["name"]["value"].clone()),
        Some(json!("Save")),
        "{tree}"
    );

    // When: a click at the button's centre
    for kind in ["mousePressed", "mouseReleased"] {
        call(
            &mut session,
            "Input.dispatchMouseEvent",
            &json!({ "type": kind, "x": 20, "y": 12, "button": "left", "clickCount": 1 }),
        );
    }

    // Then
    assert_eq!(title(&mut session), "clicked");

    // When: a key in the field
    call(
        &mut session,
        "Runtime.evaluate",
        &json!({ "expression": "document.getElementById('i').focus()" }),
    );
    call(
        &mut session,
        "Input.dispatchKeyEvent",
        &json!({ "type": "keyDown", "key": "a", "text": "a" }),
    );

    // Then
    assert_eq!(title(&mut session), "key a");

    // When: a frame
    let shot = call(
        &mut session,
        "Page.captureScreenshot",
        &json!({ "format": "png" }),
    );

    // Then: a PNG
    assert!(
        shot["data"]
            .as_str()
            .is_some_and(|d| d.starts_with("iVBORw0KGgo")),
        "not a PNG"
    );
}
