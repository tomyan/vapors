//! Open a window, clear it to one colour, and with `--screenshot out.png`
//! save the second frame (the first has been presented by then) and exit.
//! `cargo xtask screenshot clear_window out.png` runs this under Xvfb.

use std::path::PathBuf;
use std::process::ExitCode;

use vapors_gpu_harness::read_rgba8;
use vapors_platform::{Frame, Next, WindowOptions, run};
use vapors_scene::Color;

const CLEAR: Color = Color {
    r: 12.0 / 255.0,
    g: 200.0 / 255.0,
    b: 34.0 / 255.0,
    a: 1.0,
};

fn main() -> ExitCode {
    let screenshot = screenshot_path();
    let options = WindowOptions {
        title: "vapors: clear window".into(),
        width: 640,
        height: 400,
    };
    let result = run(options, move |frame| on_frame(frame, screenshot.as_deref()));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("clear_window failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn screenshot_path() -> Option<PathBuf> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--screenshot" {
            return args.next().map(PathBuf::from);
        }
    }
    None
}

fn on_frame(frame: &Frame<'_>, screenshot: Option<&std::path::Path>) -> Next {
    clear(frame);
    if frame.index == 0 {
        println!("first frame presented");
    }
    match screenshot {
        Some(path) if frame.index >= 1 => {
            save(frame, path);
            Next::Exit
        }
        _ => Next::Continue,
    }
}

fn clear(frame: &Frame<'_>) {
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = frame
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("clear"),
        });
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: f64::from(CLEAR.r),
                    g: f64::from(CLEAR.g),
                    b: f64::from(CLEAR.b),
                    a: f64::from(CLEAR.a),
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    frame.queue.submit([encoder.finish()]);
}

fn save(frame: &Frame<'_>, path: &std::path::Path) {
    let pixels = read_rgba8(frame.device, frame.queue, frame.texture);
    let (width, height) = (frame.texture.width(), frame.texture.height());
    let file = std::fs::File::create(path).expect("create screenshot file");
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("png header");
    writer.write_image_data(&pixels).expect("png data");
    let centre = ((height / 2) * width + width / 2) as usize * 4;
    println!(
        "screenshot written {} {width}x{height} centre={},{},{} adapter={}",
        path.display(),
        pixels[centre],
        pixels[centre + 1],
        pixels[centre + 2],
        frame.adapter.get_info().name
    );
}
