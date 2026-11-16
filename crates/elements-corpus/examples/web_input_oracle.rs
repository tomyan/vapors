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

/// The field demo's input, in the coordinates of its 640x400 canvas:
/// the card is centred, its row 24 px high at y 188, the field x 260
/// to 380; x 370 is right of "Warm", so the caret goes to its end.
const FIELD_AT: (f64, f64) = (370.0, 200.0);

/// A point on the field's first letter, in the canvas's coordinates:
/// its text starts at x 263, past its 1 px border and 2 px padding.
const FIELD_TEXT_AT: (f64, f64) = (266.0, 200.0);

/// The canvas size the points above are laid out for.
const CANVAS: (f64, f64) = (640.0, 400.0);

/// The demo page's viewport: the canvas's 400 px under the demo bar's
/// 40 (crates/web-demo/src/bar.rs).
const DEMO_VIEWPORT: (u32, u32) = (640, 440);

/// The canvas's place and size on the page.
const CANVAS_RECT: &str = "(() => { const r = document.getElementById('vapors').getBoundingClientRect(); return [r.left, r.top, r.width, r.height]; })()";

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
    let rect = evaluate(
        &mut session,
        "(() => { const r = i.getBoundingClientRect(); return [r.left + 6, r.top + r.height / 2]; })()",
    )?;
    let input_text = (
        rect[0].as_f64().unwrap_or(0.0),
        rect[1].as_f64().unwrap_or(0.0),
    );
    let chromium = play(&mut session, INPUT, input_text)?;
    let (width, height) = DEMO_VIEWPORT;
    let metrics = serde_json::json!({ "width": width, "height": height, "deviceScaleFactor": 1, "mobile": false });
    session.call("Emulation.setDeviceMetricsOverride", &metrics, DEADLINE)?;
    session.navigate(&format!("{origin}/index.html?demo=field"), DEADLINE)?;
    let origin = click_field(&mut session)?;
    let vapors = play(&mut session, MIRROR, on_page(origin, FIELD_TEXT_AT))?;
    let differ = differences(&chromium, &vapors);
    if differ.is_empty() {
        Ok(())
    } else {
        Err(differ.join("\n"))
    }
}

/// Clicks the field until the page's textarea mirrors it, "Warm" with
/// the caret at its end; the page draws its first frame meanwhile. The
/// points are the canvas's, moved by where the canvas is on the page
/// (below the demo bar); the canvas's top left on the page.
fn click_field(session: &mut Session) -> Result<(f64, f64), String> {
    let until = Instant::now() + READY;
    let wanted: State = ("Warm".to_owned(), 4, 4);
    let mut last = Err("no textarea yet".to_owned());
    while Instant::now() < until {
        let origin = match canvas_origin(session)? {
            Ok(origin) => origin,
            Err(size) => {
                last = Err(size);
                pause(session)?;
                continue;
            }
        };
        for kind in ["mousePressed", "mouseReleased"] {
            let (x, y) = on_page(origin, FIELD_AT);
            let params = serde_json::json!({ "type": kind, "x": x, "y": y, "button": "left", "clickCount": 1 });
            session.call("Input.dispatchMouseEvent", &params, DEADLINE)?;
        }
        pause(session)?;
        last = state(session, MIRROR);
        if last.as_ref() == Ok(&wanted) {
            return Ok(origin);
        }
    }
    Err(format!(
        "the page's textarea did not mirror the field as {wanted:?} within {} s of clicks at {FIELD_AT:?} on the canvas; it held {last:?}",
        READY.as_secs()
    ))
}

/// The canvas's top left on the page, once it is laid out at
/// [`CANVAS`]; else the size it has, as the reason to wait.
fn canvas_origin(session: &mut Session) -> Result<Result<(f64, f64), String>, String> {
    let rect = evaluate(session, CANVAS_RECT)?;
    let at = |i: usize| rect[i].as_f64().unwrap_or(f64::NAN);
    let size = (at(2), at(3));
    if size != CANVAS {
        return Ok(Err(format!(
            "the canvas is {size:?}, not the {CANVAS:?} its points are laid out for; with the demo bar it fits in {DEMO_VIEWPORT:?}"
        )));
    }
    Ok(Ok((at(0), at(1))))
}

/// `point`, in the canvas's coordinates, on the page.
fn on_page(origin: (f64, f64), point: (f64, f64)) -> (f64, f64) {
    (origin.0 + point.0, origin.1 + point.1)
}

/// Half a second of the page's own time.
fn pause(session: &mut Session) -> Result<(), String> {
    evaluate(
        session,
        "new Promise((done) => setTimeout(() => done(true), 500))",
    )
    .map(drop)
}
