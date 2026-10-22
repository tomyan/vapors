//! The wgpu side of a window: instance, adapter, device, configured surface.

use std::sync::Arc;

use winit::window::Window;

use crate::PlatformError;

/// A window's GPU resources.
pub struct Gpu {
    /// The adapter chosen by the `vapors-gpu` rule among those that can
    /// present here; on the web, the one the browser offers.
    pub adapter: wgpu::Adapter,
    /// Device for the app's work.
    pub device: wgpu::Device,
    /// Queue for the app's submissions.
    pub queue: wgpu::Queue,
    /// The window's surface.
    pub surface: wgpu::Surface<'static>,
    /// Current configuration; reapplied on resize.
    pub config: wgpu::SurfaceConfiguration,
}

#[cfg(not(target_family = "wasm"))]
const BACKENDS: wgpu::Backends = wgpu::Backends::PRIMARY;
/// WebGPU only: no WebGL fallback, so a browser without WebGPU fails by name.
#[cfg(target_family = "wasm")]
const BACKENDS: wgpu::Backends = wgpu::Backends::BROWSER_WEBGPU;

impl Gpu {
    /// Create everything needed to draw into `window`. Async because the
    /// browser answers adapter and device requests with promises; native
    /// callers block on it.
    ///
    /// # Errors
    ///
    /// When no adapter can present to the window, the device cannot be
    /// opened, or the surface offers no 8-bit non-sRGB format.
    pub async fn for_window(window: &Arc<Window>) -> Result<Self, PlatformError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: BACKENDS,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance
            .create_surface(Arc::clone(window))
            .map_err(|e| PlatformError::Surface(e.to_string()))?;
        let adapter = choose_adapter(&instance, &surface).await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("vapors window"),
                ..Default::default()
            })
            .await
            .map_err(|e| PlatformError::Device(e.to_string()))?;
        let size = window.inner_size();
        let config = configuration(&surface, &adapter, size.width.max(1), size.height.max(1))?;
        surface.configure(&device, &config);
        Ok(Self {
            adapter,
            device,
            queue,
            surface,
            config,
        })
    }

    /// Resize the surface. Zero-sized windows are configured as 1x1.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
    }

    /// The adapter's name and backend, for a status line. Browsers do not
    /// name their adapter, so the web says so instead of showing nothing.
    #[must_use]
    pub fn adapter_summary(&self) -> String {
        let info = self.adapter.get_info();
        let name = if info.name.is_empty() {
            "unnamed adapter"
        } else {
            &info.name
        };
        format!("{name} ({:?})", info.backend)
    }
}

/// The `vapors-gpu` rule over every adapter that can present to `surface`.
#[cfg(not(target_family = "wasm"))]
async fn choose_adapter(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'_>,
) -> Result<wgpu::Adapter, PlatformError> {
    let presentable: Vec<wgpu::Adapter> = instance
        .enumerate_adapters(BACKENDS)
        .await
        .into_iter()
        .filter(|adapter| adapter.is_surface_supported(surface))
        .collect();
    let wanted = vapors_gpu::wanted_from_env();
    Ok(vapors_gpu::select(presentable, wanted.as_deref())?)
}

/// The browser chooses: WebGPU does not enumerate adapters.
#[cfg(target_family = "wasm")]
async fn choose_adapter(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'_>,
) -> Result<wgpu::Adapter, PlatformError> {
    instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(surface),
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })
        .await
        .map_err(|e| PlatformError::NoWebGpu(e.to_string()))
}

/// The default configuration with a non-sRGB 8-bit format, so clear
/// colours round-trip to the bytes a reader expects, and `COPY_SRC` so
/// a frame can be read back for a screenshot.
fn configuration(
    surface: &wgpu::Surface<'_>,
    adapter: &wgpu::Adapter,
    width: u32,
    height: u32,
) -> Result<wgpu::SurfaceConfiguration, PlatformError> {
    let capabilities = surface.get_capabilities(adapter);
    let format = pick_format(&capabilities.formats).ok_or_else(|| {
        PlatformError::Surface(format!(
            "no 8-bit non-sRGB surface format; offered {:?}",
            capabilities.formats
        ))
    })?;
    let mut config = surface
        .get_default_config(adapter, width, height)
        .ok_or_else(|| PlatformError::Surface("adapter cannot present to this surface".into()))?;
    config.format = format;
    config.usage |= wgpu::TextureUsages::COPY_SRC;
    Ok(config)
}

/// First 8-bit RGBA or BGRA format that is not sRGB-encoded.
pub fn pick_format(formats: &[wgpu::TextureFormat]) -> Option<wgpu::TextureFormat> {
    formats.iter().copied().find(|f| {
        matches!(
            f,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_a_non_srgb_eight_bit_format() {
        // Given
        let offered = [
            wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Rgba16Float,
        ];

        // When
        let picked = pick_format(&offered);

        // Then
        assert_eq!(picked, Some(wgpu::TextureFormat::Bgra8Unorm));
    }

    #[test]
    fn srgb_only_is_refused() {
        // Given
        let offered = [wgpu::TextureFormat::Bgra8UnormSrgb];

        // When
        let picked = pick_format(&offered);

        // Then
        assert_eq!(picked, None);
    }
}
