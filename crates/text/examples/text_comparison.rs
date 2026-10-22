//! The comparison page: render the fixed paragraphs with vapors-text and
//! with headless Chromium at 1x and 2x, write the PNGs and a labelled
//! composite (1x row above, 2x row below), and check the 13 px body's
//! mean ink is within the band of Chromium's at both scales.
//! `--out <dir>` (default `target`) is where the files go; `--gamma` and
//! `--contrast` override the shader parameters for tuning.
//!
//! ```
//! cargo run -p vapors-text --example text_comparison -- --out target
//! ```

#[cfg(not(target_family = "wasm"))]
mod native {
    use std::path::{Path, PathBuf};

    use vapors_text::comparison::{
        Chromium, FONT_FILES, INK_BAND, PAGE_HEIGHT, PAGE_WIDTH, Rgba, SCHEMES, block_rect, blocks,
        body_measure_rect, compose, html, ink_within_band, mean_ink, render_labels, render_page,
        row_width, write_png,
    };
    use vapors_text::{BlendParams, FontSystem};

    /// The scales rendered: 1x with light hinting, 2x with none.
    const SCALES: [u32; 2] = [1, 2];

    pub fn run(out: &Path) -> Result<(), String> {
        std::fs::create_dir_all(out)
            .map_err(|e| format!("cannot create {}: {e}", out.display()))?;
        let gpu = vapors_gpu_harness::gpu();
        let mut system = FontSystem::bundle_only();
        let params = params_from_flags()?;
        let html_path = write_html(out)?;
        let chromium = Chromium::find().map_err(|e| e.to_string())?;
        let mut rows = Vec::new();
        for scale in SCALES {
            let ours = render_page(&gpu.device, &gpu.queue, &mut system, params, scale);
            write_png(
                &out.join(format!("text_comparison_vapors_{scale}x.png")),
                &ours,
            )?;
            let theirs = chromium
                .screenshot(
                    &html_path,
                    &out.join(format!("text_comparison_chromium_{scale}x.png")),
                    PAGE_WIDTH,
                    PAGE_HEIGHT,
                    scale,
                )
                .map_err(|e| e.to_string())?;
            rows.push((scale, ours, theirs));
        }
        write_composite(out, &gpu, &mut system, params, &chromium, &rows)?;
        for (scale, ours, theirs) in &rows {
            print_ink_table(*scale, ours, theirs);
        }
        rows.iter()
            .try_for_each(|(scale, ours, theirs)| check_band(*scale, ours, theirs))
    }

    fn write_composite(
        out: &Path,
        gpu: &vapors_gpu_harness::Gpu,
        system: &mut FontSystem,
        params: BlendParams,
        chromium: &Chromium,
        rows: &[(u32, Rgba, Rgba)],
    ) -> Result<(), String> {
        let pairs: Vec<[&Rgba; 2]> = rows.iter().map(|(_, a, b)| [a, b]).collect();
        let width = pairs.iter().map(|&p| row_width(p)).max().unwrap_or(0);
        let labels = [
            format!(
                "vapors-text  gamma {}  contrast {}  (1x above, 2x below)",
                params.gamma, params.contrast
            ),
            chromium.version(),
        ];
        let strip = render_labels(
            &gpu.device,
            &gpu.queue,
            system,
            width,
            [&labels[0], &labels[1]],
        );
        let path = out.join("text_comparison.png");
        write_png(&path, &compose(&strip, &pairs))?;
        println!("composite written {}", path.display());
        Ok(())
    }

    fn params_from_flags() -> Result<BlendParams, String> {
        let mut params = BlendParams::default();
        if let Some(gamma) = flag_value("--gamma") {
            params.gamma = gamma.parse().map_err(|e| format!("--gamma {gamma}: {e}"))?;
        }
        if let Some(contrast) = flag_value("--contrast") {
            params.contrast = contrast
                .parse()
                .map_err(|e| format!("--contrast {contrast}: {e}"))?;
        }
        Ok(params)
    }

    /// Mean ink of every block in both panels at `scale`, ours against Chromium's.
    fn print_ink_table(scale: u32, ours: &Rgba, theirs: &Rgba) {
        println!(
            "{scale}x  {:<14} {:<28} {:>7} {:>8} {:>6}",
            "panel", "block", "vapors", "chromium", "ratio"
        );
        for (panel, scheme) in SCHEMES.iter().enumerate() {
            let background = scheme.background.to_rgba8();
            for (index, block) in blocks().iter().enumerate() {
                let rect = block_rect(panel, index).map(|v| v * scale);
                let (a, b) = (
                    mean_ink(ours, rect, background),
                    mean_ink(theirs, rect, background),
                );
                let name = format!("{:?} {} px w{}", block.family, block.size, block.weight);
                println!(
                    "    {:<14} {name:<28} {a:>7.4} {b:>8.4} {:>6.3}",
                    scheme.name,
                    a / b
                );
            }
        }
    }

    fn check_band(scale: u32, ours: &Rgba, theirs: &Rgba) -> Result<(), String> {
        let rect = body_measure_rect(scale);
        let background = SCHEMES[0].background.to_rgba8();
        let (a, b) = (
            mean_ink(ours, rect, background),
            mean_ink(theirs, rect, background),
        );
        println!(
            "{scale}x 13 px body mean ink: vapors {a:.4}, chromium {b:.4}, ratio {:.3} (band {INK_BAND:?})",
            a / b
        );
        if ink_within_band(a, b) {
            Ok(())
        } else {
            Err(format!(
                "{scale}x mean ink ratio {:.3} is outside {INK_BAND:?}. Fix: look at \
                 target/text_comparison.png; the renderer has gone blank, bold or misplaced, \
                 or Chromium's fonts did not load",
                a / b
            ))
        }
    }

    fn write_html(out: &Path) -> Result<PathBuf, String> {
        for (name, bytes) in FONT_FILES {
            std::fs::write(out.join(name), bytes)
                .map_err(|e| format!("cannot write {name}: {e}"))?;
        }
        let path = out.join("text_comparison.html");
        std::fs::write(&path, html())
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        path.canonicalize()
            .map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn flag_value(flag: &str) -> Option<String> {
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            if arg == flag {
                return args.next();
            }
        }
        None
    }
}

#[cfg(not(target_family = "wasm"))]
fn main() -> std::process::ExitCode {
    let out = native::flag_value("--out").map_or_else(|| "target".into(), std::path::PathBuf::from);
    match native::run(&out) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("text_comparison: {message}");
            eprintln!(
                "reproduce: cargo run -p vapors-text --example text_comparison -- --out {}",
                out.display()
            );
            std::process::ExitCode::FAILURE
        }
    }
}

/// The page needs a GPU device and Chromium; neither is reachable from wasm.
#[cfg(target_family = "wasm")]
fn main() {}
