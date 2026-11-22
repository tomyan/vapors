//! The UI thread's side: winit's window. It draws the tick the
//! application thread last showed, sends `Toggle` for Space or the
//! script's request, and wakes only for the application thread's frame
//! wakes: every frame returns `Next::Wait`, or `Next::Exit` once `--check`
//! has judged or the application thread refused.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use vapors_app::AppThread;
use vapors_platform::{App, Event, Frame, Next, Platform, report_error};
use vapors_scene::Color;

use crate::app_side::{Live, ToApp, make};
use crate::check::{REPRODUCE, latencies, problems};
use crate::record::{Drawn, Shared, lock};

pub struct Window {
    app: Option<AppThread<ToApp>>,
    record: Shared,
    shown: Arc<AtomicU64>,
    check: bool,
    /// Whether the run passed, once known; read by `main`.
    outcome: Rc<Cell<Option<bool>>>,
    /// Why the application thread refused, shown at the next frame.
    refused: Option<String>,
}

impl Window {
    pub fn new(check: bool, outcome: Rc<Cell<Option<bool>>>) -> Self {
        let (record, shown) = (Shared::default(), Arc::default());
        Window {
            app: None,
            record,
            shown,
            check,
            outcome,
            refused: None,
        }
    }

    /// Starts the application thread once the loop's waker exists.
    fn start(&mut self, platform: &Platform) {
        let live = Live {
            record: Arc::clone(&self.record),
            shown: Arc::clone(&self.shown),
            waker: platform.waker(),
            running: None,
        };
        let check = self.check;
        match AppThread::start(move || make(live, check), report_error) {
            Ok(app) => self.app = Some(app),
            Err(error) => self.refused = Some(error.to_string()),
        }
        self.send(ToApp::Started, platform);
    }

    /// Sends `message`; a refusal is shown by the next frame, asked for here.
    fn send(&mut self, message: ToApp, platform: &Platform) {
        let sent = self.app.as_ref().map(|app| app.send(message));
        if let Some(Err(error)) = sent {
            self.refused = Some(error.to_string());
        }
        if self.refused.is_some() {
            platform.request_redraw();
        }
    }

    /// The turns so far, and a `Toggle` for each toggle the script asked.
    fn serve_requests(&mut self) -> bool {
        let asked = std::mem::take(&mut lock(&self.record).requests);
        let Some(app) = self.app.as_ref().filter(|_| asked > 0) else {
            return false;
        };
        lock(&self.record).turns.push(app.turns());
        for _ in 0..asked {
            if let Err(error) = app.send(ToApp::Toggle) {
                self.refused = Some(error.to_string());
            }
        }
        true
    }

    /// Stops the application thread and judges the record.
    fn judge(&mut self) -> bool {
        let app = self.app.take().expect("judged once, after the start");
        let id = app.thread_id();
        lock(&self.record).turns.push(app.turns());
        let mut found: Vec<String> = app
            .stop()
            .err()
            .map(|e| e.to_string())
            .into_iter()
            .collect();
        let record = lock(&self.record);
        found.extend(problems(&record, id));
        summary(&record);
        drop(record);
        for problem in &found {
            eprintln!("live_timer --check: {problem}");
        }
        if !found.is_empty() {
            eprintln!("live_timer --check: {REPRODUCE}");
        }
        found.is_empty()
    }

    fn draw(&self, frame: &Frame<'_>, request: bool) {
        let shown = self.shown.load(Ordering::SeqCst);
        let shade = u8::try_from(shown % 2 * 120 + 40).unwrap_or(40);
        clear(frame, Color::from_rgba8(shade, shade, 60, 255));
        let mut record = lock(&self.record);
        record.ui_thread.get_or_insert(std::thread::current().id());
        record.frames.push(Drawn {
            at: Instant::now(),
            shown,
            request,
        });
    }
}

fn summary(record: &crate::record::Record) {
    let mut waits = latencies(record);
    waits.sort();
    println!(
        "live_timer: {} ticks, {} frames, tick to frame median {:?} max {:?}, turns {:?}",
        record.ticks.len(),
        record.frames.len(),
        waits.get(waits.len() / 2),
        waits.last(),
        record.turns
    );
}

impl App for Window {
    fn event(&mut self, event: Event, platform: &Platform) {
        if let Event::Key {
            key,
            pressed: true,
            repeat: false,
            ..
        } = event
            && key == " "
        {
            self.send(ToApp::Toggle, platform);
        }
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        // the end wake's frame, like a request's, is drawn for the script
        let finished = lock(&self.record).finished();
        let request = self.serve_requests() || finished;
        self.draw(frame, request);
        if let Some(refused) = self.refused.take() {
            eprintln!("live_timer: the application thread refused: {refused}; {REPRODUCE}");
            self.outcome.set(Some(false));
            return Next::Exit;
        }
        if self.check && finished {
            let passed = self.judge();
            self.outcome.set(Some(passed));
            return Next::Exit;
        }
        Next::Wait
    }

    fn drawn(&mut self, platform: &Platform) {
        if self.app.is_none() && self.outcome.get().is_none() {
            self.start(platform);
        }
    }
}

fn clear(frame: &Frame<'_>, color: Color) {
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    vapors_renderer::clear_view(frame.device, frame.queue, &view, color);
}
