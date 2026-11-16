//! What a scene pass shares with the shapes drawn in it
//! (docs/plans/decal.md section 2): the camera uniform (group 0: the
//! view-projection and the target's size, which the shapes' margins in
//! device pixels need), a node's slot (group 1, a dynamic offset into one
//! buffer: the world matrix and a mesh's colour), and the depth and blend
//! a shape on a plane draws with.

/// Bytes in the camera uniform: a 4x4 `f32` matrix, then the target's
/// width and height in device pixels, padded to 16 bytes.
pub const CAMERA_BYTES: u64 = 80;

/// Bytes in a node's slot: its world matrix and its colour.
pub const NODE_BYTES: u64 = 80;

/// The two bind group layouts every scene pipeline is built on.
pub struct ProjectiveLayouts {
    /// Group 0, the camera.
    pub camera: wgpu::BindGroupLayout,
    /// Group 1, a node's slot by dynamic offset.
    pub node: wgpu::BindGroupLayout,
}

impl ProjectiveLayouts {
    /// Builds both layouts.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            camera: uniform_layout(device, "scene camera", false, CAMERA_BYTES),
            node: uniform_layout(device, "scene node", true, NODE_BYTES),
        }
    }
}

fn uniform_layout(
    device: &wgpu::Device,
    label: &str,
    has_dynamic_offset: bool,
    bytes: u64,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset,
                min_binding_size: wgpu::BufferSize::new(bytes),
            },
            count: None,
        }],
    })
}

/// A shape on a plane is tested against the meshes drawn before it
/// (`LessEqual`) and writes no depth, so shapes on one plane overlay in
/// paint order; `bias` lifts it off a face it lies on (decal.md section
/// 3, set from a measurement where the scene builds it).
#[must_use]
pub fn shape_depth(
    format: wgpu::TextureFormat,
    bias: wgpu::DepthBiasState,
) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format,
        depth_write_enabled: Some(false),
        depth_compare: Some(wgpu::CompareFunction::LessEqual),
        stencil: wgpu::StencilState::default(),
        bias,
    }
}

/// Colour over what is there by the shape's alpha; alpha accumulates as
/// coverage over coverage, so a shape over an opaque mesh leaves 1 and
/// one over the cleared backing leaves its own coverage.
pub const SHAPE_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
};
