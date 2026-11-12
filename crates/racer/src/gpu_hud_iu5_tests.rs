//! GPU tier: IU5 on the racer at 1100 x 720 (docs/design/
//! incremental-contracts.md, section f). While the track animates behind
//! a HUD whose values stay the same (the camera's transition while
//! paused), a frame restyles nothing, builds no text layout, has no HUD
//! write noted a change (`hud_changed` covers only the racer's writes
//! before the update), lays out no more boxes than an idle update,
//! uploads no atlas bytes, draws the HUD with the same calls and
//! renders the track once. While racing the clock changes every frame; there a frame
//! restyles nothing, renders the track once and draws with the same
//! calls. What a changed text costs (text layouts, boxes, atlas bytes)
//! does not hold yet: IU3/IU4 targets in docs/plans/3d-1c.md.
use crate::app::Racer;
use crate::gpu_frames::{frame_at, idle};
use std::time::Duration;
use vapors_app::FrameCounts;
use vapors_gpu_harness::{Gpu, gpu};
use vapors_platform::Instant;

/// A 60 Hz frame interval, as the browser's.
const INTERVAL: Duration = Duration::from_millis(16);

/// What one racer frame did to the HUD and the track.
#[derive(Debug)]
pub(crate) struct Work {
    pub(crate) restyled: usize,
    pub(crate) text_layouts: usize,
    pub(crate) laid_out: usize,
    pub(crate) hud_changed: bool,
    pub(crate) renders: u64,
    pub(crate) atlas_bytes: u64,
    /// The race clock's seconds when the frame ended.
    pub(crate) elapsed: f32,
    /// Draw calls per kind of run: shadows, quads, texts, icons,
    /// images, surfaces.
    pub(crate) draws: [u64; 6],
}

/// A racer and the time of its last frame.
pub(crate) struct Clocked {
    pub(crate) racer: Racer,
    now: Instant,
}

impl Clocked {
    pub(crate) fn idle(gpu: &Gpu) -> Self {
        let now = Instant::now();
        Self {
            racer: idle(gpu),
            now,
        }
    }

    /// One frame an interval after the last, and what it did.
    pub(crate) fn frame(&mut self, gpu: &Gpu) -> Work {
        self.now += INTERVAL;
        let before = self.racer.track.renders();
        frame_at(&mut self.racer, gpu, self.now);
        assert_eq!(self.racer.app.failure().get(), None, "the frame failed");
        work(&self.racer, self.racer.track.renders() - before)
    }

    /// `count` frames, after `settling` frames that are not returned.
    pub(crate) fn frames(&mut self, gpu: &Gpu, settling: usize, count: usize) -> Vec<Work> {
        let all: Vec<Work> = (0..settling + count).map(|_| self.frame(gpu)).collect();
        all.into_iter().skip(settling).collect()
    }
}

fn work(racer: &Racer, renders: u64) -> Work {
    let doc = racer.document.borrow();
    let counts = racer.app.frame_counts();
    Work {
        restyled: doc.elements_restyled(),
        text_layouts: doc.text_layouts_built(),
        laid_out: doc.boxes_laid_out(),
        hud_changed: racer.hud_changed,
        renders,
        atlas_bytes: counts.atlas_bytes(),
        elapsed: racer.run.race.elapsed,
        draws: draws(&counts),
    }
}

fn draws(counts: &FrameCounts) -> [u64; 6] {
    [
        counts.shadows,
        counts.quads,
        counts.texts,
        counts.icons,
        counts.images,
        counts.surfaces,
    ]
    .map(|work| work.draw_calls)
}

/// Two idle updates of a paused racer, then the racer with its camera
/// moving: its first frames show the toggled camera label and are not
/// returned.
fn camera_moving(gpu: &Gpu) -> (Clocked, [Work; 2]) {
    let mut clocked = Clocked::idle(gpu);
    let idle_updates = [clocked.frame(gpu), clocked.frame(gpu)];
    clocked.racer.command("camera").expect("toggles the camera");
    clocked.frames(gpu, 2, 0);
    assert!(clocked.racer.run.camera.moving, "the camera came to rest");
    (clocked, idle_updates)
}

#[test]
fn gpu_an_animating_frame_behind_an_unchanged_hud_does_no_hud_work() {
    // Given
    let gpu = gpu();
    let (mut clocked, [idle_update, idle_again]) = camera_moving(&gpu);

    // When
    let animating = clocked.frames(&gpu, 0, 5);

    // Then
    assert!(clocked.racer.run.camera.moving, "the camera came to rest");
    assert_eq!(
        idle_again.laid_out, idle_update.laid_out,
        "idle samples differ"
    );
    assert_eq!(idle_again.draws, idle_update.draws, "idle samples differ");
    assert_eq!(idle_update.renders, 0, "{idle_update:?}");
    for frame in &animating {
        let quiet = (frame.restyled, frame.text_layouts, frame.hud_changed);
        assert_eq!(quiet, (0, 0, false), "{frame:?}");
        assert_eq!((frame.renders, frame.atlas_bytes), (1, 0), "{frame:?}");
        assert!(
            frame.laid_out <= idle_update.laid_out,
            "{frame:?}; idle {idle_update:?}"
        );
        assert_eq!(frame.draws, idle_update.draws, "{frame:?}");
    }
}

#[test]
fn gpu_a_racing_frame_restyles_nothing_and_renders_the_track_once() {
    // Given
    let gpu = gpu();
    let mut clocked = Clocked::idle(&gpu);
    clocked.racer.command("toggle").expect("drives");

    // When: the first frames show the race's message and button label
    let racing = clocked.frames(&gpu, 2, 5);

    // Then
    assert!(!clocked.racer.run.paused, "the race is paused");
    for pair in racing.windows(2) {
        assert!(
            pair[1].elapsed > pair[0].elapsed,
            "the race clock stood: {pair:?}"
        );
    }
    for frame in &racing {
        assert_eq!((frame.restyled, frame.renders), (0, 1), "{frame:?}");
        assert_eq!(frame.draws, racing[0].draws, "{frame:?}");
    }
}
