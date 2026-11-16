//! The one door every render pipeline and shader module vapors makes
//! goes through (one-scene F0): each is counted, so a frame that builds
//! one can be found and refused. Building them is the first frame's
//! largest cost (docs/perf/scene-run-2026-11-13.md: about 76 ms of
//! pipeline compiles with Mesa's shader cache empty), so every one is
//! built before the first frame. `clippy.toml` refuses wgpu's own
//! constructors anywhere else in this workspace; touche's workspace has
//! no such ban yet (its engine builds compute pipelines directly), so
//! its pin bump adds the same entries and a counted compute door.
//!
//! The counts are process-wide: a pipeline built on another thread is
//! counted in whichever frame the reader is measuring, and pipelines
//! wgpu builds for itself (at device creation, for example) are never
//! counted. A count of none means none through this door.

use std::sync::atomic::{AtomicU64, Ordering};

/// Render pipelines and shader modules this process has built, through
/// [`render_pipeline`] and [`shader_module`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Built {
    /// Render pipelines built.
    pub pipelines: u64,
    /// Shader modules built.
    pub shader_modules: u64,
}

static PIPELINES: AtomicU64 = AtomicU64::new(0);
static SHADER_MODULES: AtomicU64 = AtomicU64::new(0);

/// What this process has built so far; a frame's difference is what it built.
#[must_use]
pub fn built() -> Built {
    Built {
        pipelines: PIPELINES.load(Ordering::Relaxed),
        shader_modules: SHADER_MODULES.load(Ordering::Relaxed),
    }
}

/// `device.create_render_pipeline(descriptor)`, counted.
#[must_use]
pub fn render_pipeline(
    device: &wgpu::Device,
    descriptor: &wgpu::RenderPipelineDescriptor<'_>,
) -> wgpu::RenderPipeline {
    PIPELINES.fetch_add(1, Ordering::Relaxed);
    #[allow(clippy::disallowed_methods)] // the one place it is called
    device.create_render_pipeline(descriptor)
}

/// `device.create_shader_module(descriptor)`, counted.
#[must_use]
pub fn shader_module(
    device: &wgpu::Device,
    descriptor: wgpu::ShaderModuleDescriptor<'_>,
) -> wgpu::ShaderModule {
    SHADER_MODULES.fetch_add(1, Ordering::Relaxed);
    #[allow(clippy::disallowed_methods)] // the one place it is called
    device.create_shader_module(descriptor)
}

impl Built {
    /// What was built between `before` and `self`.
    ///
    /// # Panics
    /// When `before` is later than `self`: the arguments are swapped.
    #[must_use]
    pub fn since(self, before: Built) -> Built {
        let later = |now: u64, then: u64| {
            now.checked_sub(then).expect(
                "Built::since: `before` is later than `self`; the counts only grow, so call \
                 later.since(earlier)",
            )
        };
        Built {
            pipelines: later(self.pipelines, before.pipelines),
            shader_modules: later(self.shader_modules, before.shader_modules),
        }
    }

    /// Nothing was built.
    #[must_use]
    pub fn is_none(self) -> bool {
        self == Built::default()
    }
}

#[cfg(test)]
#[path = "pipelines_tests.rs"]
mod tests;
