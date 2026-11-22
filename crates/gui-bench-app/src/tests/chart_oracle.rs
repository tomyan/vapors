//! Chromium as the live oracle for the chart's bars: the plot's own CSS
//! (`ui/chart_plot.wisp`'s `<style>`) with each height written inline as
//! the spec gives it (`bar_height`, `previous_height`, not the app's binding), six plots (3 series, compare off then on), laid
//! out by headless Chromium (the pinned flags of `vapors_text::comparison`,
//! 1280-wide window, scale 1). One launch; the bars' boxes come back
//! relative to their plot as `series compare class slot x y w h` lines.
use crate::dashboard::{CHART_BARS, SEGMENTS, bar_height, previous_height};
use vapors_text::comparison::Chromium;

pub(super) const REPRODUCE: &str = "cargo test -p vapors-gui-bench-app chart_heights";

/// One of Chromium's bar boxes: its state, class, slot and [x, y, w, h].
pub(super) struct Bar {
    pub series: usize,
    pub compare: bool,
    pub class: String,
    pub rect: [f32; 4],
}

fn style() -> &'static str {
    let source = include_str!("../../ui/chart_plot.wisp");
    let open = source
        .find("<style>")
        .expect("a <style> in chart_plot.wisp")
        + 7;
    &source[open..source.find("</style>").expect("a closing </style>")]
}

fn plot(series: usize, compare: bool) -> String {
    let slot = |i: usize| {
        let prev = format!(
            r#"<div class="prev" style="height: {}px"></div>"#,
            previous_height(series, i)
        );
        let cur = format!(
            r#"<div class="cur" style="height: {}px"></div>"#,
            bar_height(series, i)
        );
        format!(
            r#"<div class="slot">{}{cur}</div>"#,
            if compare { prev } else { String::new() }
        )
    };
    let cmp = if compare { " cmp" } else { "" };
    let slots: String = (0..CHART_BARS).map(slot).collect();
    format!(r#"<div class="plot{cmp}" data-state="{series} {compare}">{slots}</div>"#)
}

const SCRIPT: &str = r#"<script>
const out = [];
for (const plot of document.querySelectorAll(".plot")) {
  const o = plot.getBoundingClientRect();
  for (const bar of plot.querySelectorAll(".prev, .cur")) {
    const r = bar.getBoundingClientRect();
    const slot = [...plot.children].indexOf(bar.parentElement);
    out.push([plot.dataset.state, bar.className, slot, r.x - o.x, r.y - o.y, r.width, r.height].join(" "));
  }
}
const pre = document.createElement("pre");
pre.id = "bars";
pre.textContent = out.join("\n");
document.body.append(pre);
</script>"#;

fn page() -> String {
    let plots: String = [false, true]
        .iter()
        .flat_map(|c| (0..SEGMENTS.len()).map(move |s| plot(s, *c)))
        .collect();
    let place = ".plot { position: relative; left: 0; top: 0; }";
    format!(
        "<!doctype html><meta charset=utf-8><style>body {{ margin: 0; }} {}{place}</style><body>{plots}{SCRIPT}",
        style()
    )
}

/// Chromium's bar boxes for every series with compare off and on, from
/// one launch shared by the tests.
pub(super) fn chromium_bars() -> &'static [Bar] {
    static BARS: std::sync::OnceLock<Vec<Bar>> = std::sync::OnceLock::new();
    BARS.get_or_init(launch)
}

fn launch() -> Vec<Bar> {
    let chromium = Chromium::find().unwrap_or_else(|e| panic!("{e} ({REPRODUCE})"));
    let dir = std::env::temp_dir().join(format!("vapors-chart-heights-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a work directory under the temp dir");
    let file = dir.join("plots.html");
    std::fs::write(&file, page()).expect("the page is written");
    let dom = chromium.dump_dom(&file, 1280, 1800, 1.0);
    std::fs::remove_dir_all(&dir).expect("the work directory is removed");
    parse(&dom.unwrap_or_else(|e| panic!("{e} ({REPRODUCE})")))
}

fn parse(dom: &str) -> Vec<Bar> {
    let open = "<pre id=\"bars\">";
    let start = dom.find(open).unwrap_or_else(|| {
        panic!("Chromium's page wrote no bars: its script did not run ({REPRODUCE})")
    });
    let body = &dom[start + open.len()..];
    body[..body.find("</pre>").expect("a closed <pre>")]
        .lines()
        .map(bar)
        .collect()
}

fn bar(line: &str) -> Bar {
    let f: Vec<&str> = line.split(' ').collect();
    assert_eq!(
        f.len(),
        8,
        "Chromium's line {line:?} is not 8 fields ({REPRODUCE})"
    );
    let n = |i: usize| -> f32 {
        f[i].parse()
            .unwrap_or_else(|_| panic!("{line:?}: field {i}"))
    };
    Bar {
        series: f[0].parse().expect("a series"),
        compare: f[1] == "true",
        class: f[2].to_owned(),
        rect: [n(4), n(5), n(6), n(7)],
    }
}
