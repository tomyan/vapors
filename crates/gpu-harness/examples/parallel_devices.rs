//! A crash reproduction, kept so it is not lost; nothing runs it.
//! `cargo run -p vapors-gpu-harness --example parallel_devices -- [threads]`
//! opens and drops a device 20 times on each of `threads` threads (4 by
//! default) through `vapors_gpu_harness::gpu()`, and exits 0 if nothing
//! crashed.
//!
//! On the Debian box (Intel UHD 630 on ANV, and lavapipe; wgpu 30.0.1;
//! 2026-10-26) the process often dies with SIGSEGV partway through. The
//! same work as four `#[test]` functions in one test binary crashed in
//! 27 of 30 runs under `cargo test`, in 16 of 20 with
//! `--test-threads=2`, and in none of 20 with `--test-threads=1` or of
//! 10 under nextest, which runs each test in its own process. Selecting
//! lavapipe (`VAPORS_GPU_ADAPTER=llvmpipe`) crashed in 15 of 20. This
//! example, 20 runs each: 4 threads 19 crashed, 2 threads 13, 1 thread
//! none; lavapipe at 4 threads 11. So the GPU tiers run under nextest.
//! No backtrace yet (no gdb on the box); nothing is reported upstream.

use std::thread;

/// Devices each thread opens and drops.
const DEVICES_PER_THREAD: usize = 20;

fn main() {
    let threads = std::env::args()
        .nth(1)
        .map_or(Ok(4), |n| n.parse::<usize>())
        .expect("the thread count is a whole number, as in `-- 4`");
    let workers: Vec<_> = (0..threads).map(|_| thread::spawn(open_and_drop)).collect();
    for worker in workers {
        worker.join().expect("a worker thread panicked");
    }
    println!("{threads} threads x {DEVICES_PER_THREAD} devices: no crash");
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
