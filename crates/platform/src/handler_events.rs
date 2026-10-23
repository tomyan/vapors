//! The handler's answers to winit: resumed, the loop's own messages,
//! and window events. A lost device is checked first in each, so
//! nothing reaches the GPU or the app after it.

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
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

    fn user_event(&mut self, event_loop: &ActiveEventLoop, message: Message) {
        if self.stop_if_lost(event_loop) {
            return;
        }
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
        if let WindowEvent::Resized(size) = event
            && let Some(gpu) = self.gpu.as_mut()
        {
            gpu.resize(size.width, size.height);
            self.platform.request_redraw();
        }
        if let WindowEvent::ScaleFactorChanged { .. } = event {
            self.platform.request_redraw();
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
