//! The flags that set how headless Chromium renders a screenshot: one
//! list, passed by every launch (`Chromium::command`) and recorded by a
//! saved reference (`vapors-elements-corpus`'s `paint_reference`), so a
//! reference captured with other flags is refused as stale. The per-run
//! paths (`--screenshot`, a profile directory) are not among them.

/// Chromium's render flags for a `width` by `height` CSS pixel window
/// at `scale` device pixels per CSS pixel.
#[must_use]
pub fn render_flags(width: u32, height: u32, scale: f64) -> Vec<String> {
    vec![
        "--headless=new".to_owned(),
        "--no-sandbox".to_owned(),
        "--hide-scrollbars".to_owned(),
        format!("--force-device-scale-factor={scale}"),
        // Fonts load after first paint; give the page settled time.
        "--virtual-time-budget=5000".to_owned(),
        format!("--window-size={width},{height}"),
    ]
}

#[cfg(test)]
#[path = "chromium_flags_tests.rs"]
mod tests;
