//! Test support for the GPU tier.
//!
//! A GPU test calls [`gpu()`] for a device. Which adapter it gets is the
//! `vapors-gpu` rule: `VAPORS_GPU_ADAPTER` names it when set, otherwise
//! the first hardware adapter, otherwise the first software one. A tier
//! that cannot run fails. It never skips.

mod readback;

pub use readback::read_rgba8;

/// The adapter this process is testing on, with its device and queue.
pub struct Gpu {
    /// The adapter, for its name and limits.
    pub adapter: wgpu::Adapter,
    /// The device every test allocates from.
    pub device: wgpu::Device,
    /// The queue every test submits to.
    pub queue: wgpu::Queue,
}

/// Open the adapter the environment asks for.
///
/// # Panics
///
/// When no adapter matches, with a message listing the adapters that
/// were found and how to fix it. A panic is the right failure here: the
/// caller is a test, and the message is the report.
#[must_use]
pub fn gpu() -> Gpu {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY));
    let wanted = vapors_gpu::wanted_from_env();
    let adapter = match vapors_gpu::select(adapters, wanted.as_deref()) {
        Ok(adapter) => adapter,
        Err(error) => panic!("{error}"),
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("vapors gpu test"),
        ..Default::default()
    }))
    .expect("the selected adapter refused to open a device");
    Gpu {
        adapter,
        device,
        queue,
    }
}
