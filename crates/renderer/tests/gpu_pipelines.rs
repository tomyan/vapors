//! GPU tier: the pipeline door counts what it builds, so a frame that
//! builds a pipeline can be found (one-scene F0).

use vapors_gpu_harness::gpu;
use vapors_renderer::SolidPipeline;
use vapors_renderer::pipelines::{Built, built};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[test]
fn gpu_building_a_pipeline_counts_it_and_its_shader_module() {
    // Given
    let gpu = gpu();
    let before = built();

    // When
    let _solid = SolidPipeline::new(&gpu.device, FORMAT);

    // Then
    let expected = Built {
        pipelines: 1,
        shader_modules: 1,
    };
    assert_eq!(built().since(before), expected);
}
