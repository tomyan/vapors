//! The comparison page: render the fixed paragraphs with vapors-text and
//! with headless Chromium, write both PNGs and a labelled side-by-side
//! composite, and check the 13 px body's mean ink is within the band of
//! Chromium's. `--out <dir>` (default `target`) is where the files go;
//! `--gamma` and `--contrast` override the shader parameters for tuning.
//!
//! ```
//! cargo run -p vapors-text --example text_comparison -- --out target
//! ```

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use vapors_text::comparison::{
    Chromium, FONT_FILES, INK_BAND, PAGE_HEIGHT, PAGE_WIDTH, Rgba, SCHEMES, block_rect, blocks,
    body_measure_rect, compose, html, ink_within_band, mean_ink, render_labels, render_page,
    write_png,
};
use vapors_text::{BlendParams, FontSystem};

fn main() -> ExitCode {
    let out = flag_value("--out").map_or_else(|| PathBuf::from("target"), PathBuf::from);
    match run(&out) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("text_comparison: {message}");
            eprintln!(
                "reproduce: cargo run -p vapors-text --example text_comparison -- --out {}",
                out.display()
            );
            ExitCode::FAILURE
        }
    }
}

fn run(out: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out).map_err(|e| format!("cannot create {}: {e}", out.display()))?;
    let gpu = vapors_gpu_harness::gpu();
    let mut system = FontSystem::bundle_only();
    let params = params_from_flags()?;

    let ours = render_page(&gpu.device, &gpu.queue, &mut system, params);
    write_png(&out.join("text_comparison_vapors.png"), &ours)?;

    let html_path = write_html(out)?;
    let chromium = Chromium::find().map_err(|e| e.to_string())?;
    let theirs = chromium
        .screenshot(
            &html_path,
            &out.join("text_comparison_chromium.png"),
            PAGE_WIDTH,
            PAGE_HEIGHT,
        )
        .map_err(|e| e.to_string())?;

    let labels = [
        format!(
            "vapors-text  gamma {}  contrast {}",
            params.gamma, params.contrast
        ),
        chromium.version(),
    ];
    let strip = render_labels(
        &gpu.device,
        &gpu.queue,
        &mut system,
        PAGE_WIDTH * 2 + 8,
        [&labels[0], &labels[1]],
    );
    let composite = compose(&strip, &ours, &theirs);
    let composite_path = out.join("text_comparison.png");
    write_png(&composite_path, &composite)?;
    println!("composite written {}", composite_path.display());

    print_ink_table(&ours, &theirs);
    check_band(&ours, &theirs)
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

/// Mean ink of every block in both panels, ours against Chromium's.
fn print_ink_table(ours: &Rgba, theirs: &Rgba) {
    println!(
        "{:<14} {:<28} {:>7} {:>8} {:>6}",
        "panel", "block", "vapors", "chromium", "ratio"
    );
    for (panel, scheme) in SCHEMES.iter().enumerate() {
        let background = scheme.background.to_rgba8();
        for (index, block) in blocks().iter().enumerate() {
            let rect = block_rect(panel, index);
            let (a, b) = (
                mean_ink(ours, rect, background),
                mean_ink(theirs, rect, background),
            );
            let name = format!("{:?} {} px w{}", block.family, block.size, block.weight);
            println!(
                "{:<14} {name:<28} {a:>7.4} {b:>8.4} {:>6.3}",
                scheme.name,
                a / b
            );
        }
    }
}

fn write_html(out: &Path) -> Result<PathBuf, String> {
    for (name, bytes) in FONT_FILES {
        std::fs::write(out.join(name), bytes).map_err(|e| format!("cannot write {name}: {e}"))?;
    }
    let path = out.join("text_comparison.html");
    std::fs::write(&path, html()).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    path.canonicalize()
        .map_err(|e| format!("{}: {e}", path.display()))
}

fn check_band(
    ours: &vapors_text::comparison::Rgba,
    theirs: &vapors_text::comparison::Rgba,
) -> Result<(), String> {
    let rect = body_measure_rect();
    let background = SCHEMES[0].background.to_rgba8();
    let (a, b) = (
        mean_ink(ours, rect, background),
        mean_ink(theirs, rect, background),
    );
    println!(
        "13 px body mean ink: vapors {a:.4}, chromium {b:.4}, ratio {:.3} (band {:?})",
        a / b,
        INK_BAND
    );
    if ink_within_band(a, b) {
        Ok(())
    } else {
        Err(format!(
            "mean ink ratio {:.3} is outside {INK_BAND:?}. Fix: look at target/text_comparison.png; \
             the renderer has gone blank, bold or misplaced, or Chromium's fonts did not load",
            a / b
        ))
    }
}

fn flag_value(flag: &str) -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == flag {
            return args.next();
        }
    }
    None
}
