//! The web page's text input against Chromium's own `<input>` (E6,
//! `web_input`). Run by `scripts/web-text-input.sh`, which serves the
//! built page with `input.html` beside it and starts Chromium with
//! WebGPU; this attaches to it:
//!
//! ```text
//! cargo run -p vapors-elements-corpus --example web_input_oracle -- <profile> <origin>
//! ```
//!
//! Fails naming each step whose text or selection differs.

use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use vapors_devtools::Session;
use vapors_elements_corpus::web_input::{DEADLINE, State, differences, evaluate, play, state};

/// `<input id="i" value="Warm">`'s state.
const INPUT: &str = "[i.value, i.selectionStart, i.selectionEnd]";

/// The state the page's textarea last took from the app's field.
const MIRROR: &str = "(() => { const t = document.querySelector('textarea'); const [s, e] = (t.dataset.vaporsSelection || '').split(' ').map(Number); return [t.dataset.vaporsText, s, e]; })()";

/// The field demo's input, laid out in the 640x400 viewport this sets: the card is
/// centred, its row 24 px high at y 188, the field x 260 to 380; x 370
/// is right of "Warm", so the caret goes to its end.
const FIELD_AT: (f64, f64) = (370.0, 200.0);

/// The first frame and the field's mirror: 1.03 to 1.07 s in three runs
/// on the dev box with a warm Chrome (2026-11-01); 60 s allows for a
/// cold runner, whose Chrome took up to 57.9 s to its first page.
const READY: Duration = Duration::from_secs(60);

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [profile, origin] = args.as_slice() else {
        eprintln!(
            "FAIL web_input_oracle: usage: web_input_oracle <profile> <origin>; run scripts/web-text-input.sh target/web <dir>"
        );
        return ExitCode::FAILURE;
    };
    match run(Path::new(profile), origin) {
        Ok(()) => {
            println!(
                "ok   web_input_oracle: vapors' field held what Chromium's <input> held after every step"
            );
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!(
                "FAIL web_input_oracle: {why}\n     reproduce: scripts/web-text-input.sh target/web \"$(mktemp -d)\""
            );
            ExitCode::FAILURE
        }
    }
}

fn run(profile: &Path, origin: &str) -> Result<(), String> {
    let mut session = Session::attach(profile, DEADLINE, &mut || Ok(()))?;
    let focus = serde_json::json!({ "enabled": true });
    session.call("Emulation.setFocusEmulationEnabled", &focus, DEADLINE)?;
    let metrics =
        serde_json::json!({ "width": 640, "height": 400, "deviceScaleFactor": 1, "mobile": false });
    session.call("Emulation.setDeviceMetricsOverride", &metrics, DEADLINE)?;
    session.navigate(&format!("{origin}/input.html"), DEADLINE)?;
    evaluate(&mut session, "i.focus(), i.setSelectionRange(4, 4), true")?;
    let chromium = play(&mut session, INPUT)?;
    session.navigate(&format!("{origin}/index.html?demo=field"), DEADLINE)?;
    click_field(&mut session)?;
    let vapors = play(&mut session, MIRROR)?;
    let differ = differences(&chromium, &vapors);
    if differ.is_empty() {
        Ok(())
    } else {
        Err(differ.join("\n"))
    }
}

/// Clicks the field until the page's textarea mirrors it, "Warm" with
/// the caret at its end; the page draws its first frame meanwhile.
fn click_field(session: &mut Session) -> Result<(), String> {
    let until = Instant::now() + READY;
    let wanted: State = ("Warm".to_owned(), 4, 4);
    let mut last = Err("no textarea yet".to_owned());
    while Instant::now() < until {
        for kind in ["mousePressed", "mouseReleased"] {
            let (x, y) = FIELD_AT;
            let params = serde_json::json!({ "type": kind, "x": x, "y": y, "button": "left", "clickCount": 1 });
            session.call("Input.dispatchMouseEvent", &params, DEADLINE)?;
        }
        evaluate(
            session,
            "new Promise((done) => setTimeout(() => done(true), 500))",
        )?;
        last = state(session, MIRROR);
        if last.as_ref() == Ok(&wanted) {
            return Ok(());
        }
    }
    Err(format!(
        "the page's textarea did not mirror the field as {wanted:?} within {} s of clicks at {FIELD_AT:?}; it held {last:?}",
        READY.as_secs()
    ))
}
