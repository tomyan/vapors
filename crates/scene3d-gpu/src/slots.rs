//! One uniform slot per mesh node in one buffer, bound at the slot's
//! dynamic offset. A slot is written when it does not hold its node's
//! current world version, and not otherwise; a removed node's slot is
//! reused (`slot_table`).

use crate::FrameError;
use crate::pipeline::{MESH_BYTES, Pipeline};
use crate::slot_table::{SlotTable, capacity_for};
use vapors_app::SurfaceFrame;
use vapors_scene3d::{Colour, Mat4, NodeId};

/// Slots the first buffer holds; a full one is replaced by one holding
/// the next power of two, up to the device's `max_buffer_size`.
const FIRST_CAPACITY: u32 = 16;

pub(crate) struct Slots {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    stride: u32,
    capacity: u32,
    table: SlotTable,
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
            table: SlotTable::default(),
        }
    }

    /// Frees the slots of nodes not in `nodes` and gives each of `nodes`
    /// without a slot one, growing the buffer when full. A grown buffer
    /// holds nothing, so every slot is then unwritten.
    ///
    /// # Errors
    ///
    /// [`FrameError::TooManySlots`] past the device's `max_buffer_size`.
    pub(crate) fn assign(
        &mut self,
        device: &wgpu::Device,
        pipeline: &Pipeline,
        nodes: &[NodeId],
    ) -> Result<(), FrameError> {
        let needed = self.table.assign(nodes);
        if needed <= self.capacity {
            return Ok(());
        }
        let limit = device.limits().max_buffer_size;
        self.capacity = capacity_for(needed, self.stride, limit)?;
        (self.buffer, self.bind_group) = allocate(device, pipeline, self.stride, self.capacity);
        self.table.forget_written();
        Ok(())
    }

    /// Whether `node`'s slot holds world version `version`.
    pub(crate) fn holds(&self, node: NodeId, version: u64) -> bool {
        self.table.holds(node, version)
    }

    /// `node`'s slot's byte offset; `None` before [`Slots::assign`].
    pub(crate) fn offset(&self, node: NodeId) -> Option<u32> {
        self.table.index(node).map(|index| index * self.stride)
    }

    /// Writes `node`'s slot with `world`, at world version `version`,
    /// and `colour`, through `frame.work`; `node` has a slot from
    /// [`Slots::assign`].
    pub(crate) fn write(
        &mut self,
        frame: &SurfaceFrame<'_>,
        node: NodeId,
        version: u64,
        world: Mat4,
        colour: Colour,
    ) {
        let index = self
            .table
            .mark_written(node, version)
            .expect("every mesh node is assigned a slot before it is written");
        let floats = world.to_cols_array().into_iter().chain(colour.rgba());
        let bytes: Vec<u8> = floats.flat_map(f32::to_le_bytes).collect();
        let offset = u64::from(index * self.stride);
        frame
            .work
            .write_buffer(frame.queue, &self.buffer, offset, &bytes);
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
