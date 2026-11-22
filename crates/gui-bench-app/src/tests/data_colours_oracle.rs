//! Chromium as the live oracle for the order rows' data colours (it
//! launches on every run and panics where Chromium is absent). The page
//! is the row's avatar and pill CSS (`ui/order_row.wisp`) with each colour
//! written inline from the spec's `order(i)`, not the app's binding; the
//! computed colours come back per row and are held to the app's. Fixture:
//! orders 0..30 (all 8 avatars, all 4 pills), 1280 wide, scale 1, the
//! pinned flags of `vapors_text::comparison`. Colours are exact: no tolerance.
use super::support::*;
use crate::dashboard::order;
use vapors_text::comparison::Chromium;

const REPRODUCE: &str = "cargo test -p vapors-gui-bench-app data_colours_oracle";
const ROWS: usize = 30;

fn hex(c: u32) -> String {
    format!("#{:06x}", c & 0x00ff_ffff)
}

fn page() -> String {
    let source = include_str!("../../ui/order_row.wisp");
    let open = source.find("<style>").expect("a <style> in order_row.wisp") + 7;
    let style = &source[open..source.find("</style>").expect("a closing </style>")];
    let row = |i: usize| {
        let o = order(i);
        let (back, fore) = o.status.colors();
        format!(
            r#"<div class="order-avatar" style="background-color: {}">A</div><div class="pill" style="background-color: {}; color: {}">P</div>"#,
            hex(o.avatar),
            hex(back),
            hex(fore)
        )
    };
    let rows: String = (0..ROWS).map(row).collect();
    format!("<!doctype html><meta charset=utf-8><style>{style}</style><body>{rows}{SCRIPT}")
}

const SCRIPT: &str = r#"<script>
const c = (e, p) => getComputedStyle(e)[p].match(/\d+/g).slice(0, 3).join(" ");
const out = [...document.querySelectorAll(".order-avatar")].map((a, i) => {
  const p = document.querySelectorAll(".pill")[i];
  return [c(a, "backgroundColor"), c(p, "backgroundColor"), c(p, "color")].join(",");
});
const pre = document.createElement("pre");
pre.id = "colours";
pre.textContent = out.join("\n");
document.body.append(pre);
</script>"#;

/// Chromium's `avatar,pill background,pill text` as `r g b` per row, from
/// one launch shared by the tests.
fn chromium() -> &'static [String] {
    static ROWS_SEEN: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    ROWS_SEEN.get_or_init(launch)
}

fn launch() -> Vec<String> {
    let chromium = Chromium::find().unwrap_or_else(|e| panic!("{e} ({REPRODUCE})"));
    let dir = std::env::temp_dir().join(format!("vapors-data-colours-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a work directory");
    let file = dir.join("rows.html");
    std::fs::write(&file, page()).expect("the page is written");
    let dom = chromium.dump_dom(&file, 1280, 800, 1.0);
    std::fs::remove_dir_all(&dir).expect("the work directory is removed");
    let dom = dom.unwrap_or_else(|e| panic!("{e} ({REPRODUCE})"));
    let open = "<pre id=\"colours\">";
    let start = dom
        .find(open)
        .unwrap_or_else(|| panic!("Chromium's script wrote no colours ({REPRODUCE})"));
    let body = &dom[start + open.len()..];
    body[..body.find("</pre>").expect("a closed <pre>")]
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The app's row colours in the same form.
fn app() -> Vec<String> {
    let mounted = with_rows(ROWS);
    let doc = mounted.document.borrow();
    let get = |n, p: &str| {
        let style = doc.computed_style(n).expect("styled");
        let css = style.get_by_name(p).expect("a property").to_css();
        css.trim_start_matches("rgb(")
            .trim_end_matches(')')
            .to_owned()
    };
    let (avatars, pills) = (by_class(&doc, "order-avatar"), by_class(&doc, "pill"));
    let row = |(a, p): (_, _)| {
        format!(
            "{},{},{}",
            get(a, "background-color"),
            get(p, "background-color"),
            get(p, "color")
        )
    };
    avatars.into_iter().zip(pills).map(row).collect()
}

#[test]
fn the_app_colours_equal_chromiums_for_the_same_rows() {
    let (got, want) = (app(), chromium());
    assert_eq!(got.len(), ROWS);
    assert_eq!(
        got, want,
        "vapors rows differ from Chromium; run {REPRODUCE}"
    );
}

#[test]
#[should_panic(expected = "vapors rows differ from Chromium")]
fn a_deliberate_mismatch_fails_the_comparison() {
    let mut got = app();
    got[0] = "0 0 0,0 0 0,0 0 0".to_owned();
    assert_eq!(
        got,
        chromium(),
        "vapors rows differ from Chromium; run {REPRODUCE}"
    );
}
