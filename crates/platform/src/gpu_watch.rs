//! What the window's device is watched for from the moment it exists: its
//! loss (`device_lost.rs`) and an error no scope caught (`uncaptured.rs`).
//! Either stops the frame loop with a sentence that names it.

use winit::event_loop::EventLoopProxy;

use crate::PlatformError;
use crate::device_lost::{self, DeviceLost, Target};
use crate::uncaptured::{self, UncapturedErrors};
use crate::window::Message;

/// The states wgpu's callbacks record into, shared with the frame loop.
#[derive(Debug, Clone, Default)]
pub struct GpuWatch {
    /// The device's loss.
    pub lost: DeviceLost,
    /// The first error no scope caught.
    pub uncaptured: UncapturedErrors,
}

impl GpuWatch {
    /// Install both callbacks on `device`; each wakes the loop through
    /// `proxy` with [`Message::GpuStopped`] after recording.
    pub fn watch(&self, device: &wgpu::Device, proxy: &EventLoopProxy<Message>) {
        self.lost
            .watch(device, crate::tasks::stop_waker(proxy.clone()));
        self.uncaptured
            .watch(device, crate::tasks::stop_waker(proxy.clone()));
    }

    /// The error that stops the loop: the loss first, since an error
    /// on a lost device is a consequence of the loss.
    #[must_use]
    pub fn stop(&self, target: Target, app: &str) -> Option<PlatformError> {
        device_lost::stop_for(self.lost.get(), target, app)
            .or_else(|| uncaptured::stop_for(self.uncaptured.get(), target, app))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_lost::Lost;
    use crate::uncaptured::Uncaptured;

    #[test]
    fn a_loss_is_named_before_an_uncaptured_error_recorded_with_it() {
        // Given
        let watch = GpuWatch::default();
        watch.uncaptured.record(Uncaptured {
            kind: "Validation",
            message: "after the loss".to_owned(),
        });
        watch.lost.record(Lost {
            reason: "Unknown".to_owned(),
            message: String::new(),
        });

        // When
        let stop = watch.stop(Target::Native, "touche").expect("stops");

        // Then
        assert!(matches!(stop, PlatformError::DeviceLost(_)), "{stop}");
    }

    #[test]
    fn an_uncaptured_error_alone_stops_the_loop_naming_it() {
        // Given
        let watch = GpuWatch::default();
        assert!(watch.stop(Target::Web, "touche").is_none());
        watch.uncaptured.record(Uncaptured {
            kind: "Validation",
            message: "scissor outside the attachment".to_owned(),
        });

        // When
        let stop = watch.stop(Target::Web, "touche").expect("stops");

        // Then
        assert!(
            stop.to_string()
                .contains("(Validation: scissor outside the attachment)")
        );
    }
}
