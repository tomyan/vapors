//! Tom's chained flings on the Pixel (tests/fling-chains/, recorded with
//! scripts/fling-recorder; docs/plans/touch-momentum.md, "Boost"): each
//! link replays the previous coast from its fitted start, frame by
//! frame, then the next touch at its recorded time with its drag as
//! Chrome scrolled it (each frame's `scrollTop` change while the finger
//! was down), and releases a fling at a velocity taken from the
//! recording alone: the finger's last two recorded touch moves. The
//! booster must boost exactly the links Chrome boosted, by the previous
//! start.
//!
//! Which links Chrome boosted is read from the recordings, not from the
//! booster: a boosted coast started beyond any unboosted start (at most
//! 4046 px/s) and its start less the previous start is a release like
//! the unboosted ones (3540 to 4970 px/s). One link is a variant: 008 ->
//! 009 with the lift moved 60 ms after the drag's last frame, past the
//! 50 ms window, which Chrome's rule does not boost; no recorded link
//! lifts outside the window.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use serde_json::Value;
use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, Viewport};
use vapors_platform::Instant;
use vapors_text::FontSystem;

use super::TouchScroll;
use crate::fling::Fling;
use crate::fling_curve::FlingCurve;

/// Each link: previous recording, next, ms the lift is moved later than
/// recorded, and whether Chrome boosted the next (module doc).
const LINKS: &[(&str, &str, f64, bool)] = &[
    ("002", "003", 0.0, false),
    ("004", "005", 0.0, false),
    ("007", "008", 0.0, false),
    ("008", "009", 0.0, true),
    ("008", "009", 60.0, false),
    ("009", "010", 0.0, true),
    ("013", "014", 0.0, true),
    ("014", "015", 0.0, true),
    ("015", "016", 0.0, true),
    ("018", "019", 0.0, false),
];

fn json(name: &str, suffix: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("tests/fling-chains/fling-{name}{suffix}"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn number(value: &Value, field: &str) -> f64 {
    value[field]
        .as_f64()
        .unwrap_or_else(|| panic!("`{field}` is not a number in {value}"))
}

/// The recording's events of `kind`: time and the field `of`.
fn events(recorded: &Value, kind: &str, of: &str) -> Vec<(f64, f64)> {
    let all = recorded["events"].as_array().expect("an events list");
    all.iter()
        .filter(|e| e["kind"] == kind)
        .map(|e| (number(e, "t"), number(e, of)))
        .collect()
}

/// A page with the recorder's list at the top left, laid out.
fn page(list: &Value) -> (Rc<RefCell<Document>>, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = format!(
        "body {{ margin: 0; }} .list {{ height: {}px; overflow: auto; }} .content {{ height: {}px; }}",
        number(list, "height"),
        number(list, "scrollHeight")
    );
    doc.add_stylesheet("chain.css", &css).expect("parses");
    let (body, scroller, content) = (
        doc.body(),
        doc.create_element("div"),
        doc.create_element("div"),
    );
    doc.set_attribute(scroller, "class", "list");
    doc.set_attribute(content, "class", "content");
    doc.append(body, scroller);
    doc.append(scroller, content);
    #[allow(clippy::cast_possible_truncation, reason = "a phone list's size")]
    let viewport = Viewport {
        width: number(list, "width") as f32,
        height: number(list, "height") as f32,
        scale: 1.0,
    };
    doc.update(viewport).expect("updates");
    (Rc::new(RefCell::new(doc)), scroller)
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "page pixels and velocities, far inside f32"
)]
fn f(value: f64) -> f32 {
    value as f32
}

/// The link replayed: the velocity the next fling started at.
/// The link replayed with its lift `late` ms after the recorded one:
/// the release (from the recording) and the velocity the next fling
/// started at, and the previous start.
fn replayed(previous: &str, next: &str, late: f64) -> [f32; 3] {
    let (before, coast, after) = (
        json(previous, ".json"),
        json(previous, ".coast.json"),
        json(next, ".json"),
    );
    let (doc, list) = page(&before["list"]);
    let origin = number(&coast, "start_ms") - 100.0;
    let base = Instant::now();
    let at = |t: f64| base + Duration::from_secs_f64((t - origin) / 1000.0);
    let mut touch = TouchScroll::default();
    let previous_start = f(number(&coast, "velocity"));
    let start = number(&coast, "start_ms");
    doc.borrow_mut()
        .scroll_to(list, [0.0, f(number(&coast, "start_top"))]);
    let velocity = touch.boosted([0.0, previous_start], at(start));
    touch.fling(Fling::new(
        list,
        FlingCurve::new(velocity).expect("moving"),
        at(start),
    ));
    let down = events(&after, "touchstart", "y")[0];
    for (t, _) in events(&before, "frame", "top")
        .into_iter()
        .filter(|(t, _)| *t >= start && *t < down.0)
    {
        touch.advance(&mut doc.borrow_mut(), at(t));
    }
    drag(&mut touch, &doc, &after, down, at);
    let up = events(&after, "touchend", "y")
        .last()
        .expect("a touchend")
        .0;
    let release = released_at(&after);
    [
        release,
        touch.boosted([0.0, release], at(up + late))[1],
        previous_start,
    ]
}

/// The release velocity the recording gives, CSS px/s the scroll's way:
/// the finger's last two recorded touch positions (the start and the
/// moves Chrome delivered to the page).
fn released_at(after: &Value) -> f32 {
    let mut moves = events(after, "touchstart", "y");
    moves.extend(events(after, "touchmove", "y"));
    moves.sort_by(|a, b| a.0.total_cmp(&b.0));
    let [.., (t0, y0), (t1, y1)] = moves[..] else {
        panic!("fewer than two touch positions");
    };
    f(-(y1 - y0) / (t1 - t0) * 1000.0)
}

/// The finger down at `down` and its drag as Chrome scrolled it: each
/// frame's `scrollTop` change while the finger was down, at that frame;
/// then the lift.
fn drag(
    touch: &mut TouchScroll,
    doc: &Rc<RefCell<Document>>,
    after: &Value,
    down: (f64, f64),
    at: impl Fn(f64) -> Instant,
) {
    let list_top = number(&after["list"], "top");
    touch.down(1, [10.0, f(down.1 - list_top)], at(down.0));
    let up = events(after, "touchend", "y").last().expect("a touchend").0;
    let frames = events(after, "frame", "top");
    for pair in frames
        .windows(2)
        .filter(|pair| pair[1].0 > down.0 && pair[1].0 <= up)
    {
        touch.pan(
            &mut doc.borrow_mut(),
            [0.0, f(pair[1].1 - pair[0].1)],
            at(pair[1].0),
        );
    }
    touch.up(1, at(up));
}

#[test]
fn the_booster_boosts_the_links_chrome_boosted_and_no_others() {
    let mut found = Vec::new();
    for &(previous, next, late, chrome_boosted) in LINKS {
        let [release, started, previous_start] = replayed(previous, next, late);
        let expected = if chrome_boosted {
            release + previous_start
        } else {
            release
        };
        if (started - expected).abs() > 0.5 {
            found.push(format!(
                "{previous} -> {next} (lift {late} ms late): released at {release}, started at {started}; Chrome {}",
                if chrome_boosted { "boosted it" } else { "did not boost it" }
            ));
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
}
