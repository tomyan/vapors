//! The racer's state, grouped by what changes it.
use crate::{
    driving::Driving,
    model::{Camera, Race},
};
use vapors_platform::Instant;

/// The race, its camera, its controls and whether it is paused.
pub(crate) struct Run {
    pub(crate) race: Race,
    pub(crate) camera: Camera,
    pub(crate) driving: Driving,
    pub(crate) paused: bool,
}
impl Default for Run {
    fn default() -> Self {
        Self {
            race: Race::default(),
            camera: Camera::default(),
            driving: Driving::default(),
            paused: true,
        }
    }
}

/// Frame timing, the fixed-step remainder, the FPS average and the display scale.
pub(crate) struct Pacing {
    pub(crate) last: Option<Instant>,
    pub(crate) remainder: f32,
    pub(crate) fps_elapsed: f32,
    pub(crate) fps_frames: f32,
    pub(crate) scale: f32,
}
impl Default for Pacing {
    fn default() -> Self {
        Self {
            last: None,
            remainder: 0.0,
            fps_elapsed: 0.0,
            fps_frames: 0.0,
            scale: 1.0,
        }
    }
}

/// A fatal error, and a refused car setup.
#[derive(Default)]
pub(crate) struct Faults {
    pub(crate) error: Option<String>,
    pub(crate) setup_error: Option<String>,
}
