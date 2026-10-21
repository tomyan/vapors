//! Test support for the GPU tier.
//!
//! A GPU test calls [`gpu()`] for a device. Which adapter it gets is
//! decided by `VAPORS_GPU_ADAPTER`: when set, the adapter whose name
//! contains that text (case-insensitive) is required and anything else
//! is a failure that lists what was found; when unset, the first
//! hardware adapter is used, or the first software one if that is all
//! there is. A tier that cannot run fails. It never skips.

mod readback;
mod select;

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

/// Name of the environment variable that selects the adapter.
pub const ADAPTER_ENV: &str = "VAPORS_GPU_ADAPTER";

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
    let wanted = std::env::var(ADAPTER_ENV).ok();
    let adapter = select::adapter(adapters, wanted.as_deref());
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
