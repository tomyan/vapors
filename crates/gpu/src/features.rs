//! The optional features a device asks for: those of the wanted set
//! its adapter offers, so an adapter without them still opens one.

/// What frame timing needs (`vapors_renderer::GpuTimer`): a timestamp
/// pair written at the start and end of each render pass.
const TIMING: wgpu::Features = wgpu::Features::TIMESTAMP_QUERY;

/// The timing features among `offered`, an adapter's `features()`.
#[must_use]
pub fn timing_features(offered: wgpu::Features) -> wgpu::Features {
    TIMING & offered
}

#[cfg(test)]
mod tests {
    use super::timing_features;
    use wgpu::Features;

    #[test]
    fn timestamp_queries_are_asked_for_only_when_the_adapter_offers_them() {
        let cases = [
            (Features::empty(), Features::empty()),
            (Features::TIMESTAMP_QUERY, Features::TIMESTAMP_QUERY),
            (Features::all(), Features::TIMESTAMP_QUERY),
            (
                Features::all() - Features::TIMESTAMP_QUERY,
                Features::empty(),
            ),
        ];
        for (offered, asked) in cases {
            assert_eq!(timing_features(offered), asked, "offered {offered:?}");
        }
    }
}
