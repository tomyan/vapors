//! GPU tier: the harness runs each kind of workload and names every
//! phase it reports. Times are not asserted (docs/plans/iu1.md, PR 4).

use vapors_app::FrameTiming;
use vapors_gpu_harness::gpu_with;
use vapors_perf::gpu_time::FEATURES;
use vapors_perf::racer::RacerFrames;
use vapors_perf::report::{PHASES, section};
use vapors_perf::rows::{RowChange, Rows};
use vapors_perf::run::{Counts, Samples, Settings, Workload};
use vapors_perf::sampler::Sampler;
use vapors_platform::{App, Next};

const FEW: Settings = Settings { warmup: 1, runs: 3 };

/// Rows whose counts name the last frame changed, in `render_passes`.
struct Numbered {
    rows: Rows,
    changed: usize,
}

impl Workload for Numbered {
    fn size(&self) -> (u32, u32) {
        self.rows.size()
    }
    fn app(&mut self) -> &mut dyn App {
        self.rows.app()
    }
    fn change(&mut self, index: usize) {
        self.changed = index;
        self.rows.change(index);
    }
    fn check(&self, next: Next) -> Result<(), String> {
        self.rows.check(next)
    }
    fn counts(&self) -> Counts {
        Counts {
            render_passes: self.changed as u64,
            ..self.rows.counts()
        }
    }
    fn timing(&self) -> FrameTiming {
        self.rows.timing()
    }
}

fn sampled(workload: &mut dyn Workload) -> (Samples, bool) {
    let gpu = gpu_with(FEATURES);
    let timestamps = gpu.device.features().contains(FEATURES);
    let samples = Sampler::new(&gpu)
        .sample(workload, FEW)
        .expect("the frames ran");
    (samples, timestamps)
}

fn assert_shape(samples: &Samples, timestamps: bool) {
    assert_eq!(samples.frame.len(), FEW.runs);
    for phase in PHASES {
        let times = samples
            .phases
            .get(phase)
            .unwrap_or_else(|| panic!("no {phase} phase"));
        assert_eq!(times.len(), FEW.runs, "{phase}");
    }
    let spans = samples.gpu.as_ref().map(Vec::len);
    assert_eq!(spans, timestamps.then_some(FEW.runs));
    let times = samples.gpu_time.as_ref().map(Vec::len);
    assert_eq!(times, timestamps.then_some(FEW.runs));
}

#[test]
fn gpu_a_rows_workload_is_timed_in_every_phase_with_its_counts() {
    // Given
    let mut rows = Rows::new(100, RowChange::Text);

    // When
    let (samples, timestamps) = sampled(&mut rows);

    // Then
    assert_shape(&samples, timestamps);
    assert_eq!(samples.counts.text_layouts, 1, "{:?}", samples.counts);
    let report = section("100 rows, Text", &samples);
    for phase in PHASES {
        assert!(report.contains(&format!("| {phase} | ")), "{report}");
    }
}

#[test]
fn gpu_a_racing_racer_is_timed_in_every_phase() {
    // Given
    let mut racer = RacerFrames::new(true);

    // When
    let (samples, timestamps) = sampled(&mut racer);

    // Then
    assert_shape(&samples, timestamps);
    assert!(
        samples.counts.frame.total().draw_calls > 0,
        "{:?}",
        samples.counts
    );
}

#[test]
fn gpu_the_counts_are_the_last_timed_frames_not_the_read_back_frames() {
    // Given
    let rows = Rows::new(100, RowChange::Idle);
    let mut numbered = Numbered { rows, changed: 0 };

    // When
    let (samples, _) = sampled(&mut numbered);

    // Then: frames 1 to 3 are timed; frame 4 reads frame 3's GPU time
    assert_eq!(samples.counts.render_passes, 3);
}
