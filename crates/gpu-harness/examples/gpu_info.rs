//! Print every adapter wgpu can see and the limits the renderer cares
//! about, as Markdown. `cargo xtask gpu-info` runs this and writes
//! `docs/gpu-limits.md` from it, so the document is generated, never
//! typed.

fn main() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY));
    println!(
        "| Adapter | Type | Backend | Driver | Max texture 2D | Max buffer | f16 shaders | Storage f16 textures |"
    );
    println!("|---|---|---|---|---|---|---|---|");
    for adapter in &adapters {
        println!("{}", row(adapter));
    }
    if adapters.is_empty() {
        println!("\nNo adapters found.");
    }
}

fn row(adapter: &wgpu::Adapter) -> String {
    let info = adapter.get_info();
    let limits = adapter.limits();
    let features = adapter.features();
    format!(
        "| {} | {:?} | {:?} | {} | {} | {} MiB | {} | {} |",
        info.name,
        info.device_type,
        info.backend,
        info.driver_info,
        limits.max_texture_dimension_2d,
        limits.max_buffer_size / (1024 * 1024),
        yes_no(features.contains(wgpu::Features::SHADER_F16)),
        yes_no(features.contains(wgpu::Features::TEXTURE_FORMAT_16BIT_NORM)),
    )
}

fn yes_no(flag: bool) -> &'static str {
    if flag { "yes" } else { "no" }
}
