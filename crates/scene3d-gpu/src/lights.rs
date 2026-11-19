//! The lights uniform: the scene's ambient level and up to
//! [`MAX_LIGHTS`] lights in one buffer, written only when the bytes the
//! scene now gives differ from the ones last written. No pipeline reads
//! it but the lit pipeline (`lit.rs`).
//!
//! Layout, all colours linear light times intensity:
//! - 16 bytes: ambient `rgb`, then 1 for a perspective camera, else 0.
//! - 16 bytes: the light count as a `u32`, then the eye `xyz`: a
//!   perspective camera's world position, or an orthographic one's unit
//!   vector from the scene towards the viewer (its world +Z). A scene
//!   without lights has both zero.
//! - per light, 48 bytes in three `vec4`s: colour `rgb` and the kind
//!   (`u32`, 0 directional, 1 point); position `xyz` and range (0 for a
//!   directional); direction `xyz` (the world -Z, unit length; zeros
//!   for a point) and 0.
//!
//! Unused lights are zeros.

use crate::FrameError;
use vapors_app::SurfaceFrame;
use vapors_scene3d::{Light, MAX_LIGHTS, NodeId, NodeKind, Scene, Vec3};

/// The uniform's size: 32 bytes of header and [`MAX_LIGHTS`] of 48.
const WORDS: usize = 8 + MAX_LIGHTS * 12;
pub(crate) const LIGHTS_BYTES: u64 = WORDS as u64 * 4;

pub(crate) struct LightsUniform {
    buffer: wgpu::Buffer,
    /// What the buffer holds: zeros at first, as the device makes it,
    /// which is what a scene without lights gives.
    written: Vec<u8>,
}

impl LightsUniform {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene3d lights"),
            size: LIGHTS_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let written = vec![0; WORDS * 4];
        Self { buffer, written }
    }

    pub(crate) fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Writes the scene's lights and ambient through `frame.work`
    /// unless they are what was last written. `scene` must be updated.
    ///
    /// # Errors
    ///
    /// [`FrameError::LightDegenerate`] for a light with no usable
    /// position or direction.
    pub(crate) fn write(
        &mut self,
        frame: &SurfaceFrame<'_>,
        scene: &Scene,
    ) -> Result<(), FrameError> {
        // no light, no highlight: the eye is zeros, so a scene without
        // lights writes nothing, as before
        let eye = if scene.lights().is_empty() {
            [0.0; 4]
        } else {
            eye(scene)?
        };
        let bytes = lights_bytes(scene, eye)?;
        if bytes != self.written {
            frame
                .work
                .write_buffer(frame.queue, &self.buffer, 0, &bytes);
            self.written = bytes;
        }
        Ok(())
    }
}

/// The eye words (see the layout) of the active camera, which must
/// be rigid, as the view-projection has already checked.
pub(crate) fn eye(scene: &Scene) -> Result<[f32; 4], FrameError> {
    let node = scene.active_camera()?;
    let NodeKind::Camera { projection, .. } = scene.kind(node)? else {
        return Err(FrameError::ActiveCameraNotCamera(node));
    };
    let world = scene.world(node)?;
    if projection.is_perspective() {
        Ok(world.w_axis.truncate().extend(1.0).to_array())
    } else {
        Ok(world.z_axis.truncate().normalize().extend(0.0).to_array())
    }
}

/// The uniform's [`LIGHTS_BYTES`] for `scene` seen from `eye`.
pub(crate) fn lights_bytes(scene: &Scene, eye: [f32; 4]) -> Result<Vec<u8>, FrameError> {
    let lights = scene.lights();
    let [red, green, blue] = scene.ambient().linear();
    let count = u32::try_from(lights.len()).expect("at most 8 lights");
    let [eye_x, eye_y, eye_z, perspective] = eye;
    let head = [red, green, blue, perspective];
    let mut words: Vec<u32> = head.into_iter().map(f32::to_bits).collect();
    words.push(count);
    words.extend([eye_x, eye_y, eye_z].map(f32::to_bits));
    for (node, light) in lights {
        words.extend(light_words(scene, node, light)?);
    }
    words.resize(WORDS, 0);
    Ok(words.into_iter().flat_map(u32::to_le_bytes).collect())
}

/// One light's twelve words.
fn light_words(scene: &Scene, node: NodeId, light: Light) -> Result<[u32; 12], FrameError> {
    let world = scene.world(node)?;
    let position = world.w_axis.truncate();
    let direction = world.transform_vector3(-Vec3::Z).normalize_or_zero();
    if !position.is_finite() || direction == Vec3::ZERO {
        return Err(FrameError::LightDegenerate(node));
    }
    let (colour, intensity, kind, range) = match light {
        Light::Directional { colour, intensity } => (colour, intensity, 0, 0.0),
        Light::Point {
            colour,
            intensity,
            range,
        } => (colour, intensity, 1, range),
    };
    let [r, g, b] = colour.linear().map(|c| c * intensity);
    let (position, direction) = if kind == 1 {
        (position, Vec3::ZERO)
    } else {
        (Vec3::ZERO, direction)
    };
    let f = f32::to_bits;
    Ok([
        f(r),
        f(g),
        f(b),
        kind,
        f(position.x),
        f(position.y),
        f(position.z),
        f(range),
        f(direction.x),
        f(direction.y),
        f(direction.z),
        0,
    ])
}

#[cfg(test)]
#[path = "lights_tests.rs"]
mod tests;
