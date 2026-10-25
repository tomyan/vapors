//! A fixture as a page for Chromium: its `<style>` in the head with the
//! bundled fonts, its template in the body, and a script that writes
//! every element's border box into the DOM once fonts have loaded.

/// A fixture file split into its parts.
#[derive(Debug, PartialEq)]
pub struct Source {
    /// The `<style>` text.
    pub style: String,
    /// The template after it.
    pub template: String,
    /// The viewports from the style's `/* viewport WxH [WxH@S ..] */`
    /// comment: the fixture is laid out at each.
    pub viewports: Vec<FixtureViewport>,
}

/// One size and device scale a fixture is laid out at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixtureViewport {
    /// Width in CSS px.
    pub width: u32,
    /// Height in CSS px.
    pub height: u32,
    /// Device pixels per CSS px: the `S` of `WxH@S`, 1 without one.
    pub scale: f64,
}

/// The id of the element the page writes its boxes into.
pub const BOXES_ID: &str = "vapors-boxes";

/// Splits a fixture file.
///
/// # Errors
/// A file with a `<script>`, without exactly one `<style>`, or without a
/// `/* viewport WxH */` comment whose sizes all parse (`WxH`, or `WxH@S`
/// with S greater than 0), naming the fix.
pub fn split(name: &str, text: &str) -> Result<Source, String> {
    if text.contains("<script>") {
        return Err(format!(
            "{name}: a layout fixture has no <script>; it must load as a static page"
        ));
    }
    let start = text
        .find("<style>")
        .ok_or(format!("{name}: no <style>; a fixture starts with one"))?;
    let end = text
        .find("</style>")
        .ok_or(format!("{name}: <style> is not closed"))?;
    let style = text[start + "<style>".len()..end].to_owned();
    let viewports = viewports(&style).ok_or(format!(
        "{name}: the <style> needs a `/* viewport WxH */` comment naming the page size, such as /* viewport 400x300 */, or several, such as /* viewport 390x800 1280x800 */; a size may name a device scale greater than 0 as `/* viewport WxH@S */`, such as 640x400@1.5"
    ))?;
    let template = text[end + "</style>".len()..].to_owned();
    Ok(Source {
        style,
        template,
        viewports,
    })
}

fn viewports(style: &str) -> Option<Vec<FixtureViewport>> {
    let after = &style[style.find("/* viewport ")? + "/* viewport ".len()..];
    let sizes = &after[..after.find(" */")?];
    let viewports: Vec<FixtureViewport> = sizes
        .split_whitespace()
        .map(viewport)
        .collect::<Option<_>>()?;
    (!viewports.is_empty()).then_some(viewports)
}

fn viewport(text: &str) -> Option<FixtureViewport> {
    let (size, scale) = text.split_once('@').unwrap_or((text, "1"));
    let scale: f64 = scale.parse().ok().filter(|s: &f64| *s > 0.0)?;
    let (width, height) = size.split_once('x')?;
    Some(FixtureViewport {
        width: width.parse().ok()?,
        height: height.parse().ok()?,
        scale,
    })
}

/// The page that shows the fixture at exactly `viewport`: the fixture's
/// page in an iframe of that size, so its viewport is the iframe's.
/// Headless Chromium will not make a window narrower than about 500 px,
/// and lays a page out shorter than its window while it loads; an
/// iframe has neither limit. The fixture page writes its boxes into
/// this page (`window.parent`), where `--dump-dom` prints them.
#[must_use]
pub fn framed(source: &Source, viewport: FixtureViewport) -> String {
    let inner = html(source).replace('&', "&amp;").replace('"', "&quot;");
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><style>html, body {{ margin: 0; }} iframe {{ border: 0; display: block; }} #{BOXES_ID} {{ display: none; }}</style></head>
<body><iframe width="{w}" height="{h}" srcdoc="{inner}"></iframe></body></html>
"#,
        w = viewport.width,
        h = viewport.height,
    )
}

/// The page Chromium loads for `source`, with the font files beside it.
#[must_use]
pub fn html(source: &Source) -> String {
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8">
<style>
@font-face {{ font-family: "Geist"; src: url("Geist-wght.ttf"); font-weight: 100 900; }}
@font-face {{ font-family: "Geist Mono"; src: url("GeistMono-wght.ttf"); font-weight: 100 900; }}
html {{ font-family: Geist; }}
#{BOXES_ID} {{ display: none; }}
</style>
<style>{style}</style>
</head><body>{template}<script>
// Every @font-face is loaded, used or not, so a font that fails cannot
// pass unnoticed; the boxes are written only when all have loaded.
const write = (value) => {{
  const host = window.parent.document;
  const out = host.createElement("pre");
  out.id = "{BOXES_ID}";
  out.textContent = JSON.stringify(value);
  host.documentElement.append(out);
}};
document.fonts.ready
  .then(() => Promise.allSettled([...document.fonts].map((face) => face.load())))
  .then(() => {{
  const failed = [...document.fonts].filter((face) => face.status !== "loaded");
  if (failed.length > 0) {{
    write({{ error: "fonts did not load: " + failed.map((face) => face.family + " (" + face.status + ")").join(", ") }});
    return;
  }}
  // A fixture scrolls an element with data-scroll="x y"; Chromium
  // clamps it to the content, as vapors' Document::scroll_to does.
  for (const element of document.querySelectorAll("[data-scroll]")) {{
    const [x, y] = element.dataset.scroll.split(" ").map(Number);
    element.scrollLeft = x;
    element.scrollTop = y;
  }}
  const boxes = [];
  for (const element of document.querySelectorAll("html, body, body *")) {{
    if (element.tagName === "SCRIPT") continue;
    const r = element.getBoundingClientRect();
    boxes.push({{ tag: element.tagName.toLowerCase(), x: r.x, y: r.y, width: r.width, height: r.height }});
  }}
  const root = document.documentElement;
  write({{ viewport: [root.clientWidth, root.clientHeight], scale: devicePixelRatio, boxes }});
}});
</script></body></html>
"#,
        style = source.style,
        template = source.template,
    )
}

/// The page's JSON from a dumped DOM: `{"viewport": [w, h], "scale": s,
/// "boxes": [..]}`, the viewport and scale as the page measured them
/// (the root's `clientWidth` and `clientHeight`, and `devicePixelRatio`).
///
/// # Errors
/// When the page never wrote its boxes (a script error, or fonts that
/// never settled), naming what to look at; or when it wrote an error
/// instead, as it does when a font failed to load.
pub fn boxes_from_dom(dom: &str) -> Result<String, String> {
    let open = format!("<pre id=\"{BOXES_ID}\">");
    let start = dom.find(&open).ok_or(format!(
        "the dumped page has no <pre id=\"{BOXES_ID}\">: its script did not run or document.fonts.ready never resolved"
    ))? + open.len();
    let end = start
        + dom[start..]
            .find("</pre>")
            .ok_or("the boxes <pre> is not closed")?;
    let text = dom[start..end]
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(page) if page.get("error").is_some() => Err(format!(
            "the page refused to measure: {}; the fixture's boxes would not be Chromium's with the bundled fonts",
            page["error"]
        )),
        _ => Ok(text),
    }
}

#[cfg(test)]
#[path = "page_tests.rs"]
mod tests;
