//! Which adapter to use. The platform layer and the GPU test harness
//! both ask this crate, so a machine gives the same answer to both.
//!
//! The rule: `VAPORS_GPU_ADAPTER`, when set, names the adapter (a
//! case-insensitive substring of its name) and nothing else will do.
//! When unset, the first hardware adapter wins, then the first software
//! one. No adapter is an error that lists what was found and the fix.

mod select;

pub use select::{Candidate, NoAdapter, choose};

/// Name of the environment variable that selects the adapter.
pub const ADAPTER_ENV: &str = "VAPORS_GPU_ADAPTER";

/// The adapter name the environment asks for, if any.
#[must_use]
pub fn wanted_from_env() -> Option<String> {
    std::env::var(ADAPTER_ENV)
        .ok()
        .filter(|s| !s.trim().is_empty())
}

/// Describe adapters for selection and for error messages.
#[must_use]
pub fn candidates(adapters: &[wgpu::Adapter]) -> Vec<Candidate> {
    adapters.iter().map(Candidate::from_adapter).collect()
}

impl Candidate {
    /// A candidate from a live adapter.
    #[must_use]
    pub fn from_adapter(adapter: &wgpu::Adapter) -> Self {
        let info = adapter.get_info();
        Self {
            name: info.name,
            software: info.device_type == wgpu::DeviceType::Cpu,
            backend: format!("{:?}", info.backend),
        }
    }
}

/// Pick one adapter out of `adapters` by the rule above.
///
/// # Errors
///
/// [`NoAdapter`] when nothing matches; its `Display` is the full report.
pub fn select(
    adapters: Vec<wgpu::Adapter>,
    wanted: Option<&str>,
) -> Result<wgpu::Adapter, NoAdapter> {
    let index = choose(&candidates(&adapters), wanted)?;
    adapters
        .into_iter()
        .nth(index)
        .ok_or_else(|| NoAdapter::new(Vec::new(), wanted))
}
