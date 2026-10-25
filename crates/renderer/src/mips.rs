//! A texture with a mip chain, each level the one above averaged over
//! 2 x 2 blocks: what an image shrunk far below its size is sampled
//! from, so 1 px detail averages out as Chromium shows it rather than
//! aliasing (F2c S10 review: 1 px stripes shrunk 3x came out black and
//! white where Chromium shows grey 127).

/// An `Rgba8Unorm` texture of `rgba` (`width * height * 4` bytes) with a
/// full mip chain, for sampling with a linear mipmap filter.
///
/// # Panics
/// If `rgba` does not hold `width * height` pixels: a caller bug.
#[must_use]
pub fn upload_rgba8_mipmapped(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> wgpu::Texture {
    assert_eq!(
        rgba.len(),
        width as usize * height as usize * 4,
        "upload_rgba8_mipmapped: {width}x{height}"
    );
    let levels = chain([width, height], rgba);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("uploaded rgba8 with mips"),
        size: extent([width, height]),
        mip_level_count: u32::try_from(levels.len()).expect("a mip chain is short"),
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, (size, pixels)) in levels.iter().enumerate() {
        write_level(queue, &texture, level, *size, pixels);
    }
    texture
}

fn write_level(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    level: usize,
    size: [u32; 2],
    pixels: &[u8],
) {
    let target = wgpu::TexelCopyTextureInfo {
        mip_level: u32::try_from(level).expect("a mip chain is short"),
        ..texture.as_image_copy()
    };
    let layout = wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(size[0] * 4),
        rows_per_image: Some(size[1]),
    };
    queue.write_texture(target, pixels, layout, extent(size));
}

fn extent([width, height]: [u32; 2]) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    }
}

/// Every level from `size` down to 1 x 1.
fn chain(size: [u32; 2], rgba: &[u8]) -> Vec<([u32; 2], Vec<u8>)> {
    let mut levels = vec![(size, rgba.to_vec())];
    while let Some((size, pixels)) = levels.last().filter(|(size, _)| size[0] > 1 || size[1] > 1) {
        let next = halved(*size, pixels);
        levels.push(next);
    }
    levels
}

/// `rgba` of `size` halved on each side (never below 1), each pixel the
/// rounded average of a 2 x 2 block (1 x 2 or 2 x 1 on a side of 1).
fn halved([width, height]: [u32; 2], rgba: &[u8]) -> ([u32; 2], Vec<u8>) {
    let size = [(width / 2).max(1), (height / 2).max(1)];
    let mut out = Vec::with_capacity((size[0] * size[1] * 4) as usize);
    for y in 0..size[1] {
        for x in 0..size[0] {
            let xs = [2 * x, (2 * x + 1).min(width - 1)];
            let ys = [2 * y, (2 * y + 1).min(height - 1)];
            out.extend((0..4).map(|channel| average(rgba, width, xs, ys, channel)));
        }
    }
    (size, out)
}

/// The rounded average of `channel` over the pixels at `xs` by `ys`.
fn average(rgba: &[u8], width: u32, xs: [u32; 2], ys: [u32; 2], channel: usize) -> u8 {
    let at = |x: u32, y: u32| u32::from(rgba[(y * width + x) as usize * 4 + channel]);
    let sum: u32 = ys
        .iter()
        .flat_map(|&y| xs.iter().map(move |&x| at(x, y)))
        .sum();
    u8::try_from((sum + 2) / 4).expect("an average of bytes")
}

#[cfg(test)]
#[path = "mips_tests.rs"]
mod tests;
