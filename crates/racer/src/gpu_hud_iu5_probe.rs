//! GPU tier, ignored: prints what one racer frame costs, for the table in
//! docs/plans/3d-1c.md. Run it with
//! `cargo nextest run -p vapors-racer --run-ignored only --no-capture gpu_probe_prints_frame_costs`.
use crate::gpu_hud_iu5_tests::{Clocked, Work};
use vapors_gpu_harness::gpu;

fn print(label: &str, frames: &[Work]) {
    println!("{label}");
    for (index, frame) in frames.iter().enumerate() {
        println!(
            "  frame {index}: laid_out {} text_layouts {} atlas_bytes {} renders {}",
            frame.laid_out, frame.text_layouts, frame.atlas_bytes, frame.renders
        );
    }
}

#[test]
#[ignore = "prints numbers for docs/plans/3d-1c.md; asserts nothing"]
fn gpu_probe_prints_frame_costs() {
    let gpu = gpu();

    let mut idle = Clocked::idle(&gpu);
    print("idle update", &idle.frames(&gpu, 0, 2));

    idle.racer.command("camera").expect("toggles the camera");
    print("paused camera transition", &idle.frames(&gpu, 0, 5));

    idle.racer.run.race.speed = 100.0;
    print("camera transition, speed changed", &idle.frames(&gpu, 0, 3));

    let mut racing = Clocked::idle(&gpu);
    racing.racer.command("toggle").expect("drives");
    print("racing, 120 frames", &racing.frames(&gpu, 0, 120));
}
