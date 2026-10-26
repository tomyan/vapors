//! The a11y oracle: each fixture's page in a live headless Chromium over
//! the `DevTools` protocol, its accessibility tree once its fonts have
//! loaded, reduced (`ax_tree`) and written to or compared with
//! `fixtures/<name>.ax.json`. The tree is read at the fixture's first
//! viewport: roles, names, values and states do not change with size in
//! these fixtures.

use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use vapors_devtools::Session;
use vapors_text::comparison::{Chromium, Live};

use crate::ax_tree::reduce;
use crate::oracle::fixtures_dir;
use crate::page::{html, split};

/// Attaching and each call: vapors' `COLD_START`, 145 s, about 2.5x the
/// slowest first launch on a runner (57.9 s); a warm browser answers in
/// milliseconds.
const DEADLINE: Duration = Duration::from_secs(145);

/// The fonts' status once loading has stopped: a load that starts after
/// `document.fonts.ready` (a button's text, laid out late) is waited for
/// too, for at most 100 rounds of 50 ms.
const FONTS_SETTLED: &str = "(async () => { for (let i = 0; i < 100; i++) { await document.fonts.ready; if (document.fonts.status === 'loaded') return 'loaded'; await new Promise((r) => setTimeout(r, 50)); } return document.fonts.status; })()";

/// A live Chromium for the run, its profile under `work`.
///
/// # Errors
/// Chromium cannot start or be attached to.
pub fn start(chromium: &Chromium, work: &Path) -> Result<(Live, Session), String> {
    let profile = work.join("profile");
    let mut live = chromium
        .live(&profile, 800, 600, 1)
        .map_err(|e| e.to_string())?;
    let mut session = Session::attach(&profile, DEADLINE, &mut || live.alive())?;
    session.call("Page.enable", &json!({}), DEADLINE)?;
    Ok((live, session))
}

/// The reduced tree of fixture `name`, its page written under `work`.
///
/// # Errors
/// A fixture that does not split, a page that does not load or whose
/// fonts do not, or a tree of a shape `ax_tree` refuses.
pub fn tree(session: &mut Session, work: &Path, name: &str) -> Result<Value, String> {
    let viewport = open(session, work, name, None)?;
    let result = session.call("Accessibility.getFullAXTree", &json!({}), DEADLINE)?;
    let tree = reduce(&result).map_err(|e| format!("{name}: {e}"))?;
    Ok(json!({ "viewport": [viewport.0, viewport.1], "tree": tree }))
}

/// Fixture `name`'s page, written under `work` and opened at `size`, else
/// at the fixture's first viewport; returns the size it was opened at.
///
/// # Errors
/// A fixture that does not split, or a page that does not load or whose
/// fonts do not.
pub fn open(
    session: &mut Session,
    work: &Path,
    name: &str,
    size: Option<(u32, u32)>,
) -> Result<(u32, u32), String> {
    let path = fixtures_dir().join(format!("{name}.wisp"));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let source = split(name, &text)?;
    let first = source
        .viewports
        .first()
        .ok_or_else(|| format!("{name}: no viewport"))?;
    let (width, height) = size.unwrap_or((first.width, first.height));
    let page = work.join(format!("{name}.page.html"));
    std::fs::write(&page, html(&source)).map_err(|e| format!("{}: {e}", page.display()))?;
    let metrics = json!({ "width": width, "height": height, "deviceScaleFactor": first.scale, "mobile": false });
    session.call("Emulation.setDeviceMetricsOverride", &metrics, DEADLINE)?;
    load(session, &page)?;
    Ok((width, height))
}

/// Open `page` and wait for its load event and its fonts.
fn load(session: &mut Session, page: &Path) -> Result<(), String> {
    let url = format!("file://{}", page.display());
    session.navigate(&url, DEADLINE)?;
    let fonts = json!({ "expression": FONTS_SETTLED, "awaitPromise": true, "returnByValue": true });
    let status = session.call("Runtime.evaluate", &fonts, DEADLINE)?;
    match status["result"]["value"].as_str() {
        Some("loaded") => Ok(()),
        other => Err(format!("{url}: the fonts did not load: {other:?}")),
    }
}
