//! The UI thread's side: winit's window. It draws the tick the
//! application thread last showed, forwards Space as a toggle, and wakes
//! only for the application thread's frame wakes: every frame returns
//! `Next::Wait`, or `Next::Exit` once `--check` has judged.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use vapors_app::AppThread;
use vapors_platform::{App, Event, Frame, Next, Platform, report_error};
use vapors_scene::Color;

use crate::app_side::{Live, ToApp, make};
use crate::check::{latencies, problems};
use crate::record::{Shared, lock};

pub struct Window {
    app: Option<AppThread<ToApp>>,
    record: Shared,
    shown: Arc<AtomicU64>,
    check: bool,
    /// Whether `--check` passed, once judged; read by `main`.
    outcome: Rc<Cell<Option<bool>>>,
    /// Why the application thread could not start, shown at the next frame.
    refused: Option<String>,
}

impl Window {
    pub fn new(check: bool, outcome: Rc<Cell<Option<bool>>>) -> Self {
        Window {
            app: None,
            record: Shared::default(),
            shown: Arc::default(),
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
        self.send(ToApp::Started);
        if self.refused.is_some() {
            // a frame to show the refusal and exit; none otherwise, so Live
            // off draws nothing
            platform.request_redraw();
        }
    }

    fn send(&mut self, message: ToApp) {
        if let Some(Err(error)) = self.app.as_ref().map(|app| app.send(message)) {
            self.refused = Some(error.to_string());
        }
    }

    /// Stops the application thread and judges the record.
    fn judge(&mut self) -> bool {
        let app = self.app.take().expect("judged once, after the start");
        let id = app.thread_id();
        let mut found = app
            .stop()
            .err()
            .map(|e| vec![e.to_string()])
            .unwrap_or_default();
        let record = lock(&self.record);
        found.extend(problems(&record, id));
        let latency = latencies(&record);
        let (ticks, frames) = (record.ticks.len(), record.frames.len());
        drop(record);
        let median = latency.get(latency.len() / 2);
        println!(
            "live_timer: {ticks} ticks, {frames} frames, tick to frame median {median:?} max {:?}",
            latency.last()
        );
        for problem in &found {
            eprintln!("live_timer --check: {problem}");
        }
        found.is_empty()
    }
}

impl App for Window {
    fn event(&mut self, event: Event, _platform: &Platform) {
        if let Event::Key {
            key,
            pressed: true,
            repeat: false,
            ..
        } = event
            && key == " "
        {
            self.send(ToApp::Toggle);
        }
    }

    fn frame(&mut self, frame: &Frame<'_>) -> Next {
        let shade = u8::try_from(self.shown.load(Ordering::SeqCst) % 2 * 120 + 40).unwrap_or(40);
        clear(frame, Color::from_rgba8(shade, shade, 60, 255));
        if let Some(refused) = self.refused.take() {
            eprintln!("live_timer: the application thread refused: {refused}");
            self.outcome.set(Some(false));
            return Next::Exit;
        }
        let finished = {
            let mut record = lock(&self.record);
            record.ui_thread.get_or_insert(std::thread::current().id());
            record.frames.push(Instant::now());
            record.finished()
        };
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
