//! Test renderers: `Solid`, which fills its surface with one colour
//! (optionally marking its last column) and logs the calls it gets, and
//! `Misfit`, whose pipeline targets the wrong format.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_app::{SurfaceError, SurfaceFrame, SurfaceRenderer, SurfaceSize};
use wgpu::util::DeviceExt;

/// What a [`Solid`] was asked to do.
#[derive(Default)]
pub struct Log {
    pub mounts: usize,
    pub resizes: Vec<SurfaceSize>,
    /// The size of each render's frame.
    pub frames: Vec<SurfaceSize>,
    pub dropped: bool,
}

/// Fills its surface with `rgba`, and its last column with `edge` when
/// set; fails its renders when `fails`.
pub struct Solid {
    pub rgba: [u8; 4],
    pub edge: Option<[u8; 4]>,
    pub log: Rc<RefCell<Log>>,
    pub fails: bool,
}

impl Solid {
    /// An `rgba` renderer and its log.
    pub fn new(rgba: [u8; 4]) -> (Self, Rc<RefCell<Log>>) {
        let log = Rc::new(RefCell::new(Log::default()));
        let solid = Self {
            rgba,
            edge: None,
            log: Rc::clone(&log),
            fails: false,
        };
        (solid, log)
    }
}

impl SurfaceRenderer for Solid {
    fn mount(&mut self, _device: &wgpu::Device) -> Result<(), SurfaceError> {
        self.log.borrow_mut().mounts += 1;
        Ok(())
    }

    fn resize(&mut self, _device: &wgpu::Device, size: SurfaceSize) -> Result<(), SurfaceError> {
        self.log.borrow_mut().resizes.push(size);
        Ok(())
    }

    fn render(&mut self, mut frame: SurfaceFrame<'_>) -> Result<(), SurfaceError> {
        if self.fails {
            return Err("the track's shader did not compile".into());
        }
        self.log.borrow_mut().frames.push(frame.size);
        fill(frame.encoder, frame.view, self.rgba);
        if let Some(edge) = self.edge {
            mark_last_column(&mut frame, edge);
        }
        Ok(())
    }
}

impl Drop for Solid {
    fn drop(&mut self) {
        self.log.borrow_mut().dropped = true;
    }
}

/// Clears `view` to `rgba`.
fn fill(encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, rgba: [u8; 4]) {
    let [r, g, b, a] = rgba.map(|c| f64::from(c) / 255.0);
    let clear = wgpu::Color { r, g, b, a };
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("solid"),
        color_attachments: &[Some(attachment(view, wgpu::LoadOp::Clear(clear)))],
        ..Default::default()
    });
}

fn attachment(
    view: &wgpu::TextureView,
    load: wgpu::LoadOp<wgpu::Color>,
) -> wgpu::RenderPassColorAttachment<'_> {
    wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load,
            store: wgpu::StoreOp::Store,
        },
    }
}

const ROW: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

/// Copies `rgba` into the frame's last column, after its fill.
fn mark_last_column(frame: &mut SurfaceFrame<'_>, rgba: [u8; 4]) {
    let buffer = column_buffer(frame.device, rgba, frame.size.height);
    let extent = wgpu::Extent3d {
        width: 1,
        height: frame.size.height,
        depth_or_array_layers: 1,
    };
    let destination = last_column(frame.view.texture(), frame.size);
    frame
        .encoder
        .copy_buffer_to_texture(rows(&buffer), destination, extent);
}

/// `height` rows of `ROW` bytes, each starting with `rgba`.
fn column_buffer(device: &wgpu::Device, rgba: [u8; 4], height: u32) -> wgpu::Buffer {
    let mut bytes = vec![0; (ROW * height) as usize];
    for line in bytes.chunks_mut(ROW as usize) {
        line[..4].copy_from_slice(&rgba);
    }
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("solid edge"),
        contents: &bytes,
        usage: wgpu::BufferUsages::COPY_SRC,
    })
}

fn rows(buffer: &wgpu::Buffer) -> wgpu::TexelCopyBufferInfo<'_> {
    wgpu::TexelCopyBufferInfo {
        buffer,
        layout: wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(ROW),
            rows_per_image: None,
        },
    }
}

fn last_column(texture: &wgpu::Texture, size: SurfaceSize) -> wgpu::TexelCopyTextureInfo<'_> {
    wgpu::TexelCopyTextureInfo {
        texture,
        mip_level: 0,
        origin: wgpu::Origin3d {
            x: size.width - 1,
            y: 0,
            z: 0,
        },
        aspect: wgpu::TextureAspect::All,
    }
}
