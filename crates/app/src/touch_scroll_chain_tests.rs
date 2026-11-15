//! Tom's chained flings on the Pixel (tests/fling-chains/, recorded with
//! scripts/fling-recorder; docs/plans/touch-momentum.md, "Boost"): each
//! link replays the previous coast from its fitted start, frame by
//! frame, then the next touch at its recorded time with its drag as
//! Chrome scrolled it (each frame's `scrollTop` change while the finger
//! was down), and releases a fling. The booster must boost exactly the
//! links Chrome boosted: started at the next coast's fitted velocity,
//! fed the release that velocity implies (less the previous start when
//! Chrome boosted).

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

/// Each link: previous recording, next, and whether Chrome boosted the
/// next (its start is beyond any unboosted start in the recordings, at
/// most 3290 px/s, and its start less the previous start is a release
/// like theirs).
const LINKS: &[(&str, &str, bool)] = &[
    ("002", "003", false),
    ("004", "005", false),
    ("007", "008", false),
    ("008", "009", true),
    ("009", "010", true),
    ("013", "014", true),
    ("014", "015", true),
    ("015", "016", true),
    ("018", "019", false),
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
fn replayed(previous: &str, next: &str, boosted: bool) -> (f32, f32) {
    let (before, coast, after, next_coast) = (
        json(previous, ".json"),
        json(previous, ".coast.json"),
        json(next, ".json"),
        json(next, ".coast.json"),
    );
    let (doc, list) = page(&before["list"]);
    let origin = number(&coast, "start_ms") - 100.0;
    let base = Instant::now();
    let at = |t: f64| base + Duration::from_secs_f64((t - origin) / 1000.0);
    let mut touch = TouchScroll::default();
    let previous_start = f(number(&coast, "velocity"));
    doc.borrow_mut()
        .scroll_to(list, [0.0, f(number(&coast, "start_top"))]);
    let velocity = touch.boosted([0.0, previous_start], at(number(&coast, "start_ms")));
    touch.fling(Fling::new(
        list,
        FlingCurve::new(velocity).expect("moving"),
        at(number(&coast, "start_ms")),
    ));
    let down = events(&after, "touchstart", "y")[0];
    for (t, _) in events(&before, "frame", "top")
        .into_iter()
        .filter(|(t, _)| *t >= number(&coast, "start_ms") && *t < down.0)
    {
        touch.advance(&mut doc.borrow_mut(), at(t));
    }
    drag(&mut touch, &doc, &after, down, at);
    let up = events(&after, "touchend", "y")
        .last()
        .expect("a touchend")
        .0;
    let next_start = f(number(&next_coast, "velocity"));
    let release = if boosted {
        next_start - previous_start
    } else {
        next_start
    };
    (touch.boosted([0.0, release], at(up))[1], next_start)
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
    for &(previous, next, boosted) in LINKS {
        let (started, chrome) = replayed(previous, next, boosted);
        if (started - chrome).abs() > 0.5 {
            found.push(format!("{previous} -> {next}: started at {started}, Chrome at {chrome} (boosted: {boosted})"));
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
}
