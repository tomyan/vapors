//! Replays a fling Chrome scrolled (a transcript from
//! scripts/fling-recorder, docs/plans/touch-momentum.md) through vapors:
//! a page laid out as the recorder's list, its touches fed to the
//! platform's `Gestures` and to `ElementsApp` as the window does, an app
//! frame at each of Chrome's frames, and the list's scroll offset read
//! after each beside Chrome's `scrollTop`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a transcript's numbers are a phone page's CSS pixels, row counts and touch ids, far inside f32, usize and u64"
)]

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use serde_json::Value;
use vapors_app::ElementsApp;
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;
use vapors_gpu_harness::Gpu;
use vapors_platform::{App, Event, Frame, Gestures, Instant, Platform, Start, TouchPhase};
use vapors_renderer::clear;
use vapors_scene::Color;
use vapors_text::FontSystem;

/// The recorder's list: where it is on the page, its size, its rows.
pub struct List {
    pub top: f32,
    pub left: f32,
    pub width: f32,
    pub height: f32,
    pub row_height: f32,
    pub rows: usize,
}

/// One recorded event, its time in ms on `performance.now()`'s clock.
pub enum Recorded {
    Touch {
        t: f64,
        phase: TouchPhase,
        id: u64,
        x: f32,
        y: f32,
    },
    Frame {
        t: f64,
        top: f32,
    },
}

/// A transcript: its file, the page's density and list, its events.
pub struct Transcript {
    pub file: String,
    pub scale: f32,
    pub list: List,
    pub events: Vec<Recorded>,
}

/// One of Chrome's frames, with Chrome's scroll offset and vapors'.
pub struct Compared {
    pub t: f64,
    pub chrome: f32,
    pub ours: f32,
}

/// The transcript in `path`.
///
/// # Errors
/// A file that does not read or parse, or a field missing or of
/// another shape, naming the file and the field or event.
pub fn load(path: &Path) -> Result<Transcript, String> {
    let file = path.display().to_string();
    let text = std::fs::read_to_string(path).map_err(|e| format!("{file}: {e}"))?;
    let json: Value = serde_json::from_str(&text).map_err(|e| format!("{file}: not JSON: {e}"))?;
    let list = &json["list"];
    let number = |value: &Value, name: &str| {
        value[name]
            .as_f64()
            .ok_or_else(|| format!("{file}: `{name}` is not a number"))
    };
    let list = List {
        top: number(list, "top")? as f32,
        left: number(list, "left")? as f32,
        width: number(list, "width")? as f32,
        height: number(list, "height")? as f32,
        row_height: number(list, "rowHeight")? as f32,
        rows: number(list, "rows")? as usize,
    };
    let events = json["events"]
        .as_array()
        .ok_or_else(|| format!("{file}: no `events` list"))?;
    let events = events
        .iter()
        .enumerate()
        .filter_map(|(at, event)| {
            recorded(event)
                .map_err(|e| format!("{file}: event {at}: {e}"))
                .transpose()
        })
        .collect::<Result<_, _>>()?;
    Ok(Transcript {
        scale: number(&json, "devicePixelRatio")? as f32,
        file,
        list,
        events,
    })
}

/// `event` as a touch or a frame; `None` for one the replay does not
/// use (`scrollend`).
fn recorded(event: &Value) -> Result<Option<Recorded>, String> {
    let kind = event["kind"].as_str().ok_or("no `kind`")?;
    let number = |name: &str| {
        event[name]
            .as_f64()
            .ok_or(format!("`{name}` is not a number"))
    };
    let phase = match kind {
        "frame" => {
            return Ok(Some(Recorded::Frame {
                t: number("t")?,
                top: number("top")? as f32,
            }));
        }
        "scrollend" => return Ok(None),
        "touchstart" => TouchPhase::Started,
        "touchmove" => TouchPhase::Moved,
        "touchend" => TouchPhase::Ended,
        "touchcancel" => TouchPhase::Cancelled,
        other => return Err(format!("unknown kind `{other}`")),
    };
    Ok(Some(Recorded::Touch {
        t: number("t")?,
        phase,
        id: number("id")? as u64,
        x: number("x")? as f32,
        y: number("y")? as f32,
    }))
}

/// A page laid out as the recorder's: the list at the top left, its
/// rows each `row_height` tall; the document and the list.
fn page(list: &List) -> (Rc<RefCell<Document>>, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = format!(
        "body {{ margin: 0; }} .list {{ height: {}px; overflow: auto; }} .row {{ height: {}px; }}",
        list.height, list.row_height
    );
    doc.add_stylesheet("fling.css", &css).expect("parses");
    let body = doc.body();
    let scroller = doc.create_element("div");
    doc.set_attribute(scroller, "class", "list");
    doc.append(body, scroller);
    for _ in 0..list.rows {
        let row = doc.create_element("div");
        doc.set_attribute(row, "class", "row");
        doc.append(scroller, row);
    }
    (Rc::new(RefCell::new(doc)), scroller)
}

/// `transcript` replayed on `gpu`: each of Chrome's frames, compared.
///
/// # Errors
/// A transcript with no frame before its first touch, to lay the page
/// out on.
pub fn replay(transcript: &Transcript, gpu: &Gpu) -> Result<Vec<Compared>, String> {
    let (doc, list) = page(&transcript.list);
    let mut app = ElementsApp::new(Rc::clone(&doc));
    let (platform, mut gestures, base) = (Platform::detached(), Gestures::new(), Instant::now());
    let first = transcript
        .events
        .first()
        .ok_or(format!("{}: no events", transcript.file))?;
    let start = match first {
        Recorded::Frame { t, .. } | Recorded::Touch { t, .. } => *t,
    };
    let at = |t: f64| Duration::from_secs_f64(((t - start) / 1000.0).max(0.0));
    let (width, height) = size(transcript);
    let started = Start {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        format: wgpu::TextureFormat::Rgba8Unorm,
        width,
        height,
        scale_factor: f64::from(transcript.scale),
    };
    app.start(&started)
        .map_err(|e| format!("{}: App::start: {e}", transcript.file))?;
    frame(&mut app, gpu, transcript, base);
    let mut compared = Vec::new();
    for event in &transcript.events {
        match *event {
            Recorded::Touch { t, phase, id, x, y } => {
                let (x, y) = device(transcript, x, y);
                app.event(Event::Touch { id, phase, x, y }, &platform);
                for gesture in gestures.touch(id, phase, x, y, at(t)) {
                    app.event(gesture, &platform);
                }
            }
            Recorded::Frame { t, top } => {
                frame(&mut app, gpu, transcript, base + at(t));
                let ours = doc
                    .borrow()
                    .scroll_offset(list)
                    .map_or(f32::NAN, |offset| offset[1]);
                compared.push(Compared {
                    t,
                    chrome: top,
                    ours,
                });
            }
        }
    }
    Ok(compared)
}

/// A touch at page CSS pixels (`x`, `y`) in the replay's device pixels,
/// the list at its top left.
fn device(transcript: &Transcript, x: f32, y: f32) -> (f32, f32) {
    let list = &transcript.list;
    (
        (x - list.left) * transcript.scale,
        (y - list.top) * transcript.scale,
    )
}

/// The list's size in device pixels.
fn size(transcript: &Transcript) -> (u32, u32) {
    let scale = transcript.scale;
    let (width, height) = (
        transcript.list.width * scale,
        transcript.list.height * scale,
    );
    (width.ceil() as u32, height.ceil() as u32)
}

/// One app frame at `now`, at the list's size in device pixels.
fn frame(app: &mut ElementsApp, gpu: &Gpu, transcript: &Transcript, now: Instant) {
    let (width, height) = size(transcript);
    let texture = clear(&gpu.device, &gpu.queue, width, height, Color::default());
    app.frame(&Frame {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        texture: &texture,
        index: 0,
        scale_factor: f64::from(transcript.scale),
        now,
    });
}
