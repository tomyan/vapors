//! A crash reproduction, kept so it is not lost. CI compiles it
//! (`clippy --all-targets`); nothing runs it. It is a reproduction, not a
//! check.
//!
//! `cargo run -p vapors-gpu-harness --example parallel_devices -- [threads]`
//! opens and drops a device 20 times on each of `threads` threads (1 to
//! 16, default 4) through `vapors_gpu_harness::gpu()`. The cap: the
//! crash shows at 2 threads, and the box it was found on has 12, so more
//! than 16 adds nothing.
//!
//! Exit status: 0 when this run completed (one run of a probabilistic
//! reproduction proves little), 2 for a bad argument, 101 when a worker
//! panicked (its message is on stderr; with several workers the lines
//! interleave, and `main` stops at the first failed join), 139 from the
//! shell for the SIGSEGV this reproduces. The joins and the device waits
//! have no deadline: run it under `timeout 60`, where 124 is a hang. The
//! slowest completed run took 2.2 s (1 thread, dev build).
//!
//! Counting crashes, 139 being the crash:
//! `for i in $(seq 20); do timeout 60 cargo run -q -p vapors-gpu-harness
//! --example parallel_devices -- 4 >/dev/null 2>&1; echo $?; done | sort | uniq -c`
//!
//! Measured on the Debian box (Intel UHD 630 on Mesa ANV, and lavapipe;
//! wgpu 30.0.1; dev build; 2026-10-26, load about 28), 20 runs each by
//! that loop: 4 threads, 20 crashed; 2 threads, 11; 1 thread, none;
//! lavapipe (`VAPORS_GPU_ADAPTER=llvmpipe` before the command) at 4
//! threads, 16.
//! Earlier, the same work as four `#[test]` functions in one test binary
//! crashed in 27 of 30 runs under `cargo test`, 16 of 20 with
//! `--test-threads=2`, none of 20 with `--test-threads=1` and none of 10
//! under nextest, which runs each test in its own process. That test was
//! a scratch file, not kept, so those four rates cannot be rerun. The
//! GPU tiers run under nextest.

use std::num::NonZeroUsize;
use std::process::ExitCode;
use std::thread;

/// Devices each thread opens and drops.
const DEVICES_PER_THREAD: usize = 20;

/// The most threads; see the module doc for why.
const MAX_THREADS: usize = 16;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let threads = match thread_count(&arguments) {
        Ok(threads) => threads.get(),
        Err(why) => {
            eprintln!(
                "parallel_devices: {why}; usage: cargo run -p vapors-gpu-harness --example parallel_devices -- [threads]"
            );
            return ExitCode::from(2);
        }
    };
    let workers: Vec<_> = (0..threads).map(|_| thread::spawn(open_and_drop)).collect();
    for worker in workers {
        worker.join().expect("a worker thread panicked");
    }
    println!("{threads} threads x {DEVICES_PER_THREAD} devices: completed");
    ExitCode::SUCCESS
}

/// No argument is 4; one argument must be a whole number from 1 to
/// [`MAX_THREADS`].
fn thread_count(arguments: &[String]) -> Result<NonZeroUsize, String> {
    let text = match arguments {
        [] => return Ok(NonZeroUsize::new(4).expect("4 is not zero")),
        [text] => text,
        more => return Err(format!("one argument at most, got {more:?}")),
    };
    text.parse::<NonZeroUsize>()
        .ok()
        .filter(|n| n.get() <= MAX_THREADS)
        .ok_or_else(|| {
            format!("thread count {text:?} is not a whole number from 1 to {MAX_THREADS}")
        })
}

fn open_and_drop() {
    for _ in 0..DEVICES_PER_THREAD {
        let gpu = vapors_gpu_harness::gpu();
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("parallel devices"),
            size: 1 << 20,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue.write_buffer(&buffer, 0, &[1; 1024]);
        gpu.queue.submit([]);
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device finishes its work");
    }
}
