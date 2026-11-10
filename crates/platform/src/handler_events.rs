//! The handler's answers to winit: resumed, a frame's wait-until time
//! reached, the loop's own messages, and window events. A lost device is
//! checked first in `user_event` and `window_event`, so nothing reaches
//! the GPU or the app after it; the redraw `new_events` requests goes
//! through `window_event`, which checks.

use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::WindowId;

use super::Handler;
use crate::event::Event;
use crate::window::Message;

impl ApplicationHandler<Message> for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(error) = self.open(event_loop)
        {
            self.fail(event_loop, error);
        }
    }

    /// The time a frame asked for with `Next::WaitUntil` has come: one
    /// frame, and events alone wake the loop until that frame says more.
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if let StartCause::ResumeTimeReached { .. } = cause {
            // Guards a dropped frame at the deadline: if the surface
            // gives no texture, `redraw` returns before it sets the
            // control flow, and the spent `WaitUntil` would fire again
            // at once. Unproven: no test can make the acquire fail, and
            // `Handler` needs a real `ActiveEventLoop`.
            event_loop.set_control_flow(ControlFlow::Wait);
            self.platform.request_redraw();
            self.redraw_if_asked();
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, message: Message) {
        if self.stop_if_lost(event_loop) {
            return;
        }
        #[cfg(not(target_family = "wasm"))]
        self.ime_message();
        match message {
            Message::GpuReady(Ok(gpu)) => self.gpu_ready(gpu),
            Message::GpuReady(Err(error)) => self.fail(event_loop, error),
            Message::FileChosen { name, bytes } => self
                .app
                .event(Event::FileOpened { name, bytes }, &self.platform),
            Message::OpenRequested => self.app.event(Event::OpenRequested, &self.platform),
            // A wake with no loss recorded cannot happen: the callback
            // records before it wakes. Nothing to do if it ever did.
            Message::DeviceLost => {}
            Message::Wake => self.platform.request_redraw(),
            Message::Input(event) => self.app.event(event, &self.platform),
        }
        self.redraw_if_asked();
    }

    /// A lost device stops everything here first: a resize would
    /// reconfigure the surface on it and input would reach the app.
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if self.stop_if_lost(event_loop) {
            return;
        }
        if let WindowEvent::CloseRequested = event {
            event_loop.exit();
            return;
        }
        // a held clear before any event, the IME's own edit for its own
        #[cfg(not(target_family = "wasm"))]
        self.ime_window_event(&event);
        if let WindowEvent::Resized(size) = event
            && let Some(gpu) = self.gpu.as_mut()
        {
            gpu.resize(size.width, size.height);
            self.platform.request_redraw();
        }
        if let WindowEvent::ScaleFactorChanged { .. } = event {
            self.platform.request_redraw();
        }
        if let Some(window) = &self.window {
            self.input.set_scale(window.scale_factor());
        }
        for app_event in self.input.translate(&event) {
            self.app.event(app_event, &self.platform);
        }
        self.redraw_if_asked();
        if let WindowEvent::RedrawRequested = event {
            self.redraw(event_loop);
        }
    }
}
