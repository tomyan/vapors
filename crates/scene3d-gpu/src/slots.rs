//! One uniform slot per mesh node in one buffer, bound at the slot's
//! dynamic offset. A slot is written when it does not hold its node's
//! current world version, and not otherwise.

use crate::pipeline::{MESH_BYTES, Pipeline};
use std::collections::HashMap;
use vapors_scene3d::{Colour, Mat4, NodeId};

/// Slots the first buffer holds; a full one is replaced by one holding
/// the next power of two.
const FIRST_CAPACITY: u32 = 16;

pub(crate) struct Slots {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    stride: u32,
    capacity: u32,
    of: HashMap<NodeId, Slot>,
}

/// Where a node's uniform sits, and the world version it holds.
struct Slot {
    index: u32,
    written: Option<u64>,
}

impl Slots {
    pub(crate) fn new(device: &wgpu::Device, pipeline: &Pipeline) -> Self {
        let align = device.limits().min_uniform_buffer_offset_alignment;
        let stride = u32::try_from(wgpu::util::align_to(MESH_BYTES, u64::from(align)))
            .expect("a 256-byte-aligned 80-byte slot fits u32");
        let (buffer, bind_group) = allocate(device, pipeline, stride, FIRST_CAPACITY);
        Self {
            buffer,
            bind_group,
            stride,
            capacity: FIRST_CAPACITY,
            of: HashMap::new(),
        }
    }

    /// Gives each of `nodes` without a slot one, growing the buffer
    /// when full. A grown buffer holds nothing, so every slot is then
    /// unwritten.
    pub(crate) fn assign(&mut self, device: &wgpu::Device, pipeline: &Pipeline, nodes: &[NodeId]) {
        for node in nodes {
            let index = u32::try_from(self.of.len()).expect("fewer than 2^32 mesh nodes");
            self.of.entry(*node).or_insert(Slot {
                index,
                written: None,
            });
        }
        let needed = u32::try_from(self.of.len()).expect("fewer than 2^32 mesh nodes");
        if needed <= self.capacity {
            return;
        }
        self.capacity = needed.next_power_of_two();
        (self.buffer, self.bind_group) = allocate(device, pipeline, self.stride, self.capacity);
        self.of.values_mut().for_each(|slot| slot.written = None);
    }

    /// Whether `node`'s slot holds world version `version`.
    pub(crate) fn holds(&self, node: NodeId, version: u64) -> bool {
        self.of
            .get(&node)
            .is_some_and(|slot| slot.written == Some(version))
    }

    /// `node`'s slot's byte offset; `None` before [`Slots::assign`].
    pub(crate) fn offset(&self, node: NodeId) -> Option<u32> {
        self.of.get(&node).map(|slot| slot.index * self.stride)
    }

    /// Writes `node`'s slot with `world`, at world version `version`,
    /// and `colour`; `node` has a slot from [`Slots::assign`].
    pub(crate) fn write(
        &mut self,
        queue: &wgpu::Queue,
        node: NodeId,
        version: u64,
        world: Mat4,
        colour: Colour,
    ) {
        let slot = self
            .of
            .get_mut(&node)
            .expect("every mesh node is assigned a slot before it is written");
        slot.written = Some(version);
        let offset = slot.index * self.stride;
        let floats = world.to_cols_array().into_iter().chain(colour.rgba());
        let bytes: Vec<u8> = floats.flat_map(f32::to_le_bytes).collect();
        queue.write_buffer(&self.buffer, u64::from(offset), &bytes);
    }

    pub(crate) fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

fn allocate(
    device: &wgpu::Device,
    pipeline: &Pipeline,
    stride: u32,
    capacity: u32,
) -> (wgpu::Buffer, wgpu::BindGroup) {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scene3d mesh slots"),
        size: u64::from(stride) * u64::from(capacity),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("scene3d mesh slots"),
        layout: &pipeline.mesh_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &buffer,
                offset: 0,
                size: wgpu::BufferSize::new(MESH_BYTES),
            }),
        }],
    });
    (buffer, bind_group)
}
