//! Replays a fling transcript's coast (`fling_transcript.rs`) through
//! vapors: a page with the recorder's list, its scroll offset set to
//! where Chrome's coast began, flung at the start time and velocity
//! fitted to Chrome's first frames, an app frame at each of Chrome's
//! frames, and the list's offset read after each beside Chrome's
//! `scrollTop`; then frames on until the app stops asking for them.
//! The drag is not replayed: docs/plans/touch-momentum.md says why.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use vapors_app::ElementsApp;
use vapors_dom::{Dom, NodeId};
use vapors_elements::Document;
use vapors_gpu_harness::Gpu;
use vapors_platform::{App, Frame, Instant, Next, Platform, Start};
use vapors_renderer::clear;
use vapors_scene::Color;
use vapors_text::FontSystem;

use super::fling_transcript::{List, Transcript};

/// One of Chrome's frames, with Chrome's scroll offset and vapors'.
pub struct Compared {
    pub t: f64,
    pub chrome: f32,
    pub ours: f32,
}

/// The replay: Chrome's frames compared, and at every frame drawn
/// (Chrome's, then on until the app went idle or `SETTLE_FRAMES`
/// passed) vapors' offset and what the app asked for next.
pub struct Replayed {
    pub compared: Vec<Compared>,
    pub frames: Vec<(f32, Next)>,
}

/// Frames drawn past Chrome's last, at 60 Hz, waiting for the app to go
/// idle: 2 s, longer than any recorded coast runs past its recording.
const SETTLE_FRAMES: u32 = 120;
const FRAME: Duration = Duration::from_micros(16_667);

/// The app showing the page, on the transcript's clock.
struct Rig<'a> {
    app: ElementsApp,
    doc: Rc<RefCell<Document>>,
    list: NodeId,
    gpu: &'a Gpu,
    transcript: &'a Transcript,
    /// The instant of the transcript's `origin` ms.
    base: Instant,
    origin: f64,
}

impl Rig<'_> {
    fn at(&self, t: f64) -> Instant {
        self.base + Duration::from_secs_f64((t - self.origin) / 1000.0)
    }

    fn offset(&self) -> f32 {
        self.doc
            .borrow()
            .scroll_offset(self.list)
            .map_or(f32::NAN, |o| o[1])
    }

    /// One app frame at `now`, at the list's size in device pixels.
    fn frame(&mut self, now: Instant) -> Next {
        let (width, height) = size(self.transcript);
        let texture = clear(
            &self.gpu.device,
            &self.gpu.queue,
            width,
            height,
            Color::default(),
        );
        self.app.frame(&Frame {
            adapter: &self.gpu.adapter,
            device: &self.gpu.device,
            queue: &self.gpu.queue,
            texture: &texture,
            index: 0,
            scale_factor: f64::from(self.transcript.scale),
            now,
            resumed: false,
        })
    }
}

/// `transcript`'s coast replayed on `gpu`.
///
/// # Errors
/// `App::start` refused, the list not scrolling to the coast's start,
/// or the fling refused.
pub fn replay(transcript: &Transcript, gpu: &Gpu) -> Result<Replayed, String> {
    let mut rig = flung(transcript, gpu)?;
    let (mut compared, mut frames) = (Vec::new(), Vec::new());
    for &(t, chrome) in &transcript.frames {
        let next = rig.frame(rig.at(t));
        compared.push(Compared {
            t,
            chrome,
            ours: rig.offset(),
        });
        frames.push((rig.offset(), next));
    }
    settle(&mut rig, &mut frames);
    Ok(Replayed { compared, frames })
}

/// The app started on the page, drawn once, the list at the coast's
/// start and flung.
fn flung<'a>(transcript: &'a Transcript, gpu: &'a Gpu) -> Result<Rig<'a>, String> {
    let (file, coast) = (&transcript.file, &transcript.coast);
    let (doc, list) = page(&transcript.list);
    let mut rig = Rig {
        app: ElementsApp::new(Rc::clone(&doc)),
        doc,
        list,
        gpu,
        transcript,
        base: Instant::now(),
        // the clock starts 50 ms before the coast
        origin: coast.first_frame_ms.min(coast.start_ms) - 50.0,
    };
    rig.app
        .start(&start(transcript, gpu))
        .map_err(|e| format!("{file}: App::start: {e}"))?;
    rig.frame(rig.base);
    if !rig.doc.borrow_mut().scroll_to(list, [0.0, coast.start_top]) {
        return Err(format!(
            "{file}: the list did not scroll to {}",
            coast.start_top
        ));
    }
    let at = rig.at(coast.start_ms);
    rig.app
        .fling(list, [0.0, coast.velocity], at, &Platform::detached())
        .map_err(|e| format!("{file}: {e}"))?;
    Ok(rig)
}

/// Frames past Chrome's last, at 60 Hz, until the app asks for none.
fn settle(rig: &mut Rig<'_>, frames: &mut Vec<(f32, Next)>) {
    let last = rig.at(rig.transcript.frames.last().map_or(rig.origin, |f| f.0));
    for n in 1..=SETTLE_FRAMES {
        let next = rig.frame(last + FRAME * n);
        frames.push((rig.offset(), next));
        if next == Next::Wait {
            return;
        }
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
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a phone list's size, positive and small"
)]
fn size(transcript: &Transcript) -> (u32, u32) {
    let (scale, list) = (transcript.scale, &transcript.list);
    (
        (list.width * scale).ceil() as u32,
        (list.height * scale).ceil() as u32,
    )
}
