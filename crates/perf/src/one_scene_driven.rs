//! The one-scene example (`vapors-one-scene`) as the judged run's
//! workload: driven by its own script before each frame (a click on the
//! light due), and checked after each that every mesh is drawn as its
//! schedule says and the frame asked for the next. One that did not ends
//! the run.

use vapors_app::FrameTiming;
use vapors_one_scene::OneScene;
use vapors_one_scene::confirm::confirm;
use vapors_one_scene::schedule::FRAME;
use vapors_one_scene::script::drive;
use vapors_platform::{App, Next};

use crate::run::{Counts, Workload};
use crate::sampler::INTERVAL;

const _: () = assert!(
    FRAME.as_nanos() == INTERVAL.as_nanos(),
    "the example's schedule and the sampler must step one frame clock"
);
const _: () = assert!(
    vapors_one_scene::schedule::RUN_FRAMES == crate::driver::RUN_FRAMES,
    "the example's script and the judged run must be one length"
);

/// The example, animating for ever, timing its frames on the GPU.
pub struct DrivenScene {
    scene: OneScene,
    frame: usize,
    fault: Option<String>,
}

impl DrivenScene {
    /// The example, mounted.
    ///
    /// # Errors
    /// It cannot be mounted.
    pub fn new() -> Result<Self, String> {
        let mut scene = OneScene::new(None)?;
        scene.time_frames(true);
        Ok(Self {
            scene,
            frame: 0,
            fault: None,
        })
    }
}

impl Workload for DrivenScene {
    fn size(&self) -> (u32, u32) {
        vapors_one_scene::SIZE
    }

    fn change(&mut self, sample: usize) {
        self.frame = sample;
        if self.fault.is_none() {
            self.fault = drive(&mut self.scene, sample).err();
        }
    }

    fn app(&mut self) -> &mut dyn App {
        &mut self.scene
    }

    fn ready(&self) -> Result<(), String> {
        self.fault.clone().map_or(Ok(()), Err)
    }

    /// Every frame draws what the schedule says and asks for the next.
    fn check(&self, next: Next) -> Result<(), String> {
        confirm(&self.scene, self.frame)?;
        match next {
            Next::Continue => Ok(()),
            other => Err(format!(
                "frame {}: the scene stopped, asking for {other:?}",
                self.frame
            )),
        }
    }

    fn counts(&self) -> Counts {
        let document = self.scene.document().borrow();
        Counts::of(
            &document,
            self.scene.frame_counts(),
            self.scene.render_passes(),
        )
    }

    fn timing(&self) -> FrameTiming {
        self.scene.frame_timing()
    }
}
