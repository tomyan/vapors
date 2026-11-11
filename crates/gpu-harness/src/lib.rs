//! Test support for the GPU tier.
//!
//! A GPU test calls [`gpu()`] for a device. Which adapter it gets is the
//! `vapors-gpu` rule: `VAPORS_GPU_ADAPTER` names it when set, otherwise
//! the first hardware adapter, otherwise the first software one. A tier
//! that cannot run fails. It never skips.
//!
//! The tiers split on the test's name: the gpu tier runs `gpu_*`, the
//! unit tier the rest. xtask's unit tier sets [`TIER_ENV`] to
//! [`REFUSED_TIER`], and [`gpu()`] refuses there, so a GPU test without
//! the prefix fails on a machine with a GPU too, not only on a runner
//! without one. Only [`gpu()`] is guarded: a test that opens its own
//! `wgpu::Instance`, as `vapors-platform`'s `surface.rs` does for a
//! window, is not. Calling [`allowed_in`] there would extend the guard.

pub use vapors_renderer::read_rgba8;

/// Set by xtask's unit tier (`steps_tiers.rs`).
pub const TIER_ENV: &str = "VAPORS_TEST_TIER";
/// The value of [`TIER_ENV`] in which [`gpu()`] refuses.
pub const REFUSED_TIER: &str = "unit";

/// The adapter this process is testing on, with its device and queue.
pub struct Gpu {
    /// The adapter, for its name and limits.
    pub adapter: wgpu::Adapter,
    /// The device every test allocates from.
    pub device: wgpu::Device,
    /// The queue every test submits to.
    pub queue: wgpu::Queue,
}

/// Open the adapter the environment asks for, with the window's limits
/// (the defaults, three raised to the adapter's; see `vapors_gpu`).
///
/// # Panics
///
/// In the unit tier, naming the fix.
///
/// When no adapter matches, with a message listing the adapters that
/// were found and how to fix it. A panic is the right failure here: the
/// caller is a test, and the message is the report.
#[must_use]
pub fn gpu() -> Gpu {
    gpu_with(wgpu::Features::empty())
}

/// As [`gpu()`], with those of `wanted` the adapter has: the device's
/// `features()` say which. For a reader of optional features, such as
/// the timing harness's timestamp queries (`crates/perf`).
///
/// # Panics
///
/// As [`gpu()`].
#[must_use]
pub fn gpu_with(wanted: wgpu::Features) -> Gpu {
    if let Err(refusal) = allowed_in(std::env::var(TIER_ENV).ok().as_deref()) {
        panic!("{refusal}");
    }
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY));
    let named = vapors_gpu::wanted_from_env();
    let adapter = match vapors_gpu::select(adapters, named.as_deref()) {
        Ok(adapter) => adapter,
        Err(error) => panic!("{error}"),
    };
    let required_limits = match vapors_gpu::raised_limits(&adapter.limits()) {
        Ok(limits) => limits,
        Err(error) => panic!("{error}"),
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("vapors gpu test"),
        required_features: wanted & adapter.features(),
        required_limits,
        ..Default::default()
    }))
    .expect("the selected adapter refused to open a device");
    Gpu {
        adapter,
        device,
        queue,
    }
}

impl Gpu {
    /// The same adapter opened again with `max_buffer_size` lowered to
    /// `limit`, for a test of what refuses a buffer over it.
    ///
    /// # Panics
    ///
    /// If the adapter will not open the device.
    #[must_use]
    pub fn with_max_buffer_size(&self, limit: u64) -> Gpu {
        let required_limits = wgpu::Limits {
            max_buffer_size: limit,
            ..self.device.limits()
        };
        let descriptor = wgpu::DeviceDescriptor {
            label: Some("vapors gpu test, small buffers"),
            required_limits,
            ..Default::default()
        };
        let (device, queue) = pollster::block_on(self.adapter.request_device(&descriptor))
            .unwrap_or_else(|error| panic!("the adapter refused max_buffer_size {limit}: {error}"));
        Gpu {
            adapter: self.adapter.clone(),
            device,
            queue,
        }
    }
}

/// Refuse the unit tier; any other tier, or none (a test run directly
/// with `cargo nextest run`), may open a GPU.
///
/// # Errors
///
/// `tier` is `unit`, with a message naming the fix.
pub fn allowed_in(tier: Option<&str>) -> Result<(), String> {
    if tier != Some(REFUSED_TIER) {
        return Ok(());
    }
    Err(format!(
        "a test opened the GPU in the unit tier ({TIER_ENV}={REFUSED_TIER}); the gpu tier runs \
         test functions named gpu_*, so on a runner without a GPU this test fails. \
         Fix: rename the test function to gpu_<name>, then run `cargo xtask test --tier gpu`"
    ))
}

#[cfg(test)]
#[path = "tier_tests.rs"]
mod tests;
