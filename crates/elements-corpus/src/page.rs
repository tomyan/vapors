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
    /// The viewport from the style's `/* viewport WxH */` comment.
    pub viewport: [u32; 2],
}

/// The id of the element the page writes its boxes into.
pub const BOXES_ID: &str = "vapors-boxes";

/// Splits a fixture file.
///
/// # Errors
/// A file with a `<script>`, without exactly one `<style>`, or without a
/// `/* viewport WxH */` comment, naming the fix.
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
    let viewport = viewport(&style).ok_or(format!(
        "{name}: the <style> needs a `/* viewport WxH */` comment naming the page size, such as /* viewport 400x300 */"
    ))?;
    let template = text[end + "</style>".len()..].to_owned();
    Ok(Source {
        style,
        template,
        viewport,
    })
}

fn viewport(style: &str) -> Option<[u32; 2]> {
    let after = &style[style.find("/* viewport ")? + "/* viewport ".len()..];
    let size = &after[..after.find(" */")?];
    let (width, height) = size.split_once('x')?;
    Some([width.parse().ok()?, height.parse().ok()?])
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
</style>
<style>{style}</style>
</head><body>{template}<script>
document.fonts.ready.then(() => {{
  const boxes = [];
  for (const element of document.querySelectorAll("html, body, body *")) {{
    if (element.tagName === "SCRIPT") continue;
    const r = element.getBoundingClientRect();
    boxes.push({{ tag: element.tagName.toLowerCase(), x: r.x, y: r.y, width: r.width, height: r.height }});
  }}
  const out = document.createElement("pre");
  out.id = "{BOXES_ID}";
  out.textContent = JSON.stringify(boxes);
  document.documentElement.append(out);
}});
</script></body></html>
"#,
        style = source.style,
        template = source.template,
    )
}

/// The boxes JSON from a dumped DOM.
///
/// # Errors
/// When the page never wrote its boxes (a script error, or fonts that
/// never loaded), naming what to look at.
pub fn boxes_from_dom(dom: &str) -> Result<String, String> {
    let open = format!("<pre id=\"{BOXES_ID}\">");
    let start = dom.find(&open).ok_or(format!(
        "the dumped page has no <pre id=\"{BOXES_ID}\">: its script did not run or document.fonts.ready never resolved"
    ))? + open.len();
    let end = start
        + dom[start..]
            .find("</pre>")
            .ok_or("the boxes <pre> is not closed")?;
    Ok(dom[start..end]
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&"))
}

#[cfg(test)]
#[path = "page_tests.rs"]
mod tests;
