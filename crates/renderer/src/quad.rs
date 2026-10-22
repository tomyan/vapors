//! One textured quad in clip space: the vertex type and the geometry the
//! image and solid pipelines share.

use vapors_scene::Size;

/// One vertex: viewport position in clip space, texture coordinate.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Vertex {
    pub(crate) position: [f32; 2],
    pub(crate) uv: [f32; 2],
}

pub(crate) const VERTEX_COUNT: u32 = 6;
pub(crate) const VERTEX_SIZE: u64 = (std::mem::size_of::<Vertex>()) as u64;

impl Vertex {
    /// The vertex buffer layout every quad pipeline binds at slot 0.
    pub(crate) fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
            wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];
        wgpu::VertexBufferLayout {
            array_stride: VERTEX_SIZE,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

/// Two triangles covering `bounds` (top-left, bottom-right in viewport
/// pixels), in clip space, with texture coordinates 0..1 across them.
pub(crate) fn quad_vertices(
    bounds: [[f32; 2]; 2],
    viewport: Size,
) -> [Vertex; VERTEX_COUNT as usize] {
    let [[left, top], [right, bottom]] = bounds;
    let corner = |x: f32, y: f32, u: f32, v: f32| Vertex {
        position: to_clip([x, y], viewport),
        uv: [u, v],
    };
    [
        corner(left, top, 0.0, 0.0),
        corner(right, top, 1.0, 0.0),
        corner(left, bottom, 0.0, 1.0),
        corner(right, top, 1.0, 0.0),
        corner(right, bottom, 1.0, 1.0),
        corner(left, bottom, 0.0, 1.0),
    ]
}

/// Viewport pixels (origin top-left, y down) to clip space (-1..1, y up).
#[allow(clippy::cast_precision_loss)] // pixel counts are far below 2^24
pub(crate) fn to_clip(point: [f32; 2], viewport: Size) -> [f32; 2] {
    [
        point[0] / viewport.width as f32 * 2.0 - 1.0,
        1.0 - point[1] / viewport.height as f32 * 2.0,
    ]
}

pub(crate) fn vertex_bytes(vertices: &[Vertex; VERTEX_COUNT as usize]) -> Vec<u8> {
    vertices
        .iter()
        .flat_map(|v| [v.position[0], v.position[1], v.uv[0], v.uv[1]])
        .flat_map(f32::to_le_bytes)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_corners_map_to_clip_corners() {
        // Given
        let viewport = Size {
            width: 640,
            height: 400,
        };

        // When
        let top_left = to_clip([0.0, 0.0], viewport);
        let bottom_right = to_clip([640.0, 400.0], viewport);

        // Then
        assert_eq!(top_left, [-1.0, 1.0]);
        assert_eq!(bottom_right, [1.0, -1.0]);
    }

    #[test]
    fn quad_covers_the_bounds_with_matching_texture_coordinates() {
        // Given
        let viewport = Size {
            width: 100,
            height: 100,
        };

        // When
        let quad = quad_vertices([[0.0, 0.0], [50.0, 100.0]], viewport);

        // Then: first vertex is the top-left at uv 0,0; fifth is bottom-right at uv 1,1
        assert_eq!(quad[0].position, [-1.0, 1.0]);
        assert_eq!(quad[0].uv, [0.0, 0.0]);
        assert_eq!(quad[4].position, [0.0, -1.0]);
        assert_eq!(quad[4].uv, [1.0, 1.0]);
    }
}
