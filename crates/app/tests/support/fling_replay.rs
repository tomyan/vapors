//! Replays the coast of a fling Chrome scrolled (a transcript from
//! scripts/fling-recorder and its `.coast.json` from `fit.py coast`;
//! docs/plans/touch-momentum.md) through vapors: a page with the
//! recorder's list, its scroll offset set to where Chrome's coast began,
//! flung at the start time and velocity fitted to Chrome's frames, an
//! app frame at each of Chrome's coast frames, and the list's offset read
//! after each beside Chrome's `scrollTop`. The drag is not replayed: the
//! plan says why.

#![allow(
    clippy::cast_possible_truncation,
    reason = "a transcript's numbers are a phone page's CSS pixels and milliseconds, far inside f32"
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
use vapors_platform::{App, Frame, Instant, Platform, Start};
use vapors_renderer::clear;
use vapors_scene::Color;
use vapors_text::FontSystem;

/// The recorder's list: its size and its content's height, CSS px.
pub struct List {
    pub width: f32,
    pub height: f32,
    pub scroll_height: f32,
}

/// Where and how fast Chrome's coast began (`fit.py coast`), and its
/// first frame, ms on the transcript's clock and CSS px.
pub struct Coast {
    pub first_frame_ms: f64,
    pub start_ms: f64,
    pub start_top: f32,
    pub velocity: f32,
}

/// A transcript's coast: its file, the page's density and list, the
/// fitted start, and Chrome's frames from the coast's first on.
pub struct Transcript {
    pub file: String,
    pub scale: f32,
    pub list: List,
    pub coast: Coast,
    /// Chrome's frames: time in ms, `scrollTop`.
    pub frames: Vec<(f64, f32)>,
}

/// One of Chrome's frames, with Chrome's scroll offset and vapors'.
pub struct Compared {
    pub t: f64,
    pub chrome: f32,
    pub ours: f32,
}

fn json(path: &Path) -> Result<Value, String> {
    let file = path.display();
    let text = std::fs::read_to_string(path).map_err(|e| format!("{file}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{file}: not JSON: {e}"))
}

fn number(file: &str, value: &Value, name: &str) -> Result<f64, String> {
    value[name]
        .as_f64()
        .ok_or_else(|| format!("{file}: `{name}` is not a number"))
}

/// The transcript in `path` and its coast beside it
/// (`<name>.coast.json`).
///
/// # Errors
/// A file that does not read or parse, a field missing or of another
/// shape, or an event of a kind the recorder does not write, naming the
/// file and the field or event.
pub fn load(path: &Path) -> Result<Transcript, String> {
    let file = path.display().to_string();
    let (recorded, fitted) = (json(path)?, json(&path.with_extension("coast.json"))?);
    let list = &recorded["list"];
    let coast = Coast {
        first_frame_ms: number(&file, &fitted, "first_frame_ms")?,
        start_ms: number(&file, &fitted, "start_ms")?,
        start_top: number(&file, &fitted, "start_top")? as f32,
        velocity: number(&file, &fitted, "velocity")? as f32,
    };
    let events = recorded["events"]
        .as_array()
        .ok_or_else(|| format!("{file}: no `events` list"))?;
    let mut frames = Vec::new();
    for (at, event) in events.iter().enumerate() {
        if let Some(frame) =
            recorded_frame(event).map_err(|e| format!("{file}: event {at}: {e}"))?
        {
            frames.push(frame);
        }
    }
    frames.retain(|(t, _)| *t >= coast.first_frame_ms);
    Ok(Transcript {
        scale: number(&file, &recorded, "devicePixelRatio")? as f32,
        list: List {
            width: number(&file, list, "width")? as f32,
            height: number(&file, list, "height")? as f32,
            scroll_height: number(&file, list, "scrollHeight")? as f32,
        },
        coast,
        frames,
        file,
    })
}

/// `event`'s time and `scrollTop` when it is a frame; `None` for the
/// other kinds the recorder writes.
fn recorded_frame(event: &Value) -> Result<Option<(f64, f32)>, String> {
    let number = |name: &str| {
        event[name]
            .as_f64()
            .ok_or(format!("`{name}` is not a number"))
    };
    match event["kind"].as_str().ok_or("no `kind`")? {
        "frame" => Ok(Some((number("t")?, number("top")? as f32))),
        "scrollend" | "touchstart" | "touchmove" | "touchend" | "touchcancel" => Ok(None),
        other => Err(format!("unknown kind `{other}`")),
    }
}

/// A page with the recorder's list at the top left. The list holds one
/// block as tall as Chrome's content: its 400 rows were 48 px plus a
/// border Chrome drew 2 device px thick (19 491 px at 2.75), and the
/// scroll range is what the coast meets.
fn page(list: &List) -> (Rc<RefCell<Document>>, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    let css = format!(
        "body {{ margin: 0; }} .list {{ height: {}px; overflow: auto; }} .content {{ height: {}px; }}",
        list.height, list.scroll_height
    );
    doc.add_stylesheet("fling.css", &css).expect("parses");
    let (body, scroller, content) = (
        doc.body(),
        doc.create_element("div"),
        doc.create_element("div"),
    );
    doc.set_attribute(scroller, "class", "list");
    doc.set_attribute(content, "class", "content");
    doc.append(body, scroller);
    doc.append(scroller, content);
    (Rc::new(RefCell::new(doc)), scroller)
}

/// `transcript`'s coast replayed on `gpu`: each of Chrome's frames,
/// compared.
///
/// # Errors
/// `App::start` refused, the list not a scroll container, or the fling
/// refused.
pub fn replay(transcript: &Transcript, gpu: &Gpu) -> Result<Vec<Compared>, String> {
    let file = &transcript.file;
    let (doc, list) = page(&transcript.list);
    let mut app = ElementsApp::new(Rc::clone(&doc));
    let (platform, base) = (Platform::detached(), Instant::now());
    // the clock starts 50 ms before the coast's first frame
    let origin = transcript
        .coast
        .first_frame_ms
        .min(transcript.coast.start_ms)
        - 50.0;
    let at = |t: f64| base + Duration::from_secs_f64((t - origin) / 1000.0);
    app.start(&start(transcript, gpu))
        .map_err(|e| format!("{file}: App::start: {e}"))?;
    frame(&mut app, gpu, transcript, base);
    let coast = &transcript.coast;
    if !doc.borrow_mut().scroll_to(list, [0.0, coast.start_top]) {
        return Err(format!(
            "{file}: the list did not scroll to {}",
            coast.start_top
        ));
    }
    app.fling(list, [0.0, coast.velocity], at(coast.start_ms), &platform)
        .map_err(|e| format!("{file}: {e}"))?;
    let mut compared = Vec::new();
    for &(t, chrome) in &transcript.frames {
        frame(&mut app, gpu, transcript, at(t));
        let ours = doc
            .borrow()
            .scroll_offset(list)
            .map_or(f32::NAN, |offset| offset[1]);
        compared.push(Compared { t, chrome, ours });
    }
    Ok(compared)
}

fn start<'a>(transcript: &Transcript, gpu: &'a Gpu) -> Start<'a> {
    let (width, height) = size(transcript);
    Start {
        adapter: &gpu.adapter,
        device: &gpu.device,
        queue: &gpu.queue,
        format: wgpu::TextureFormat::Rgba8Unorm,
        width,
        height,
        scale_factor: f64::from(transcript.scale),
    }
}

/// The list's size in device pixels.
#[allow(clippy::cast_sign_loss, reason = "a list's size is positive")]
fn size(transcript: &Transcript) -> (u32, u32) {
    let scale = transcript.scale;
    let list = &transcript.list;
    (
        (list.width * scale).ceil() as u32,
        (list.height * scale).ceil() as u32,
    )
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
