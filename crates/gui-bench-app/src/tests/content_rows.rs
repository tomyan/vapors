//! The spec's rectangles for the top bar, the content and the metric cards.
#![allow(clippy::cast_precision_loss)] // indexes and counts are small
use super::layout::{Row, at, from_x, row};
use crate::dashboard::*;

fn bar_rows() -> Vec<Row> {
    let bar = at(SIDEBAR_WIDTH, 0.0, WINDOW.0 - SIDEBAR_WIDTH, TOPBAR_HEIGHT);
    let search = at(SEARCH_X, 14.0, SEARCH_WIDTH, CONTROL_HEIGHT);
    let export = at(EXPORT_X, 14.0, EXPORT_WIDTH, CONTROL_HEIGHT);
    let avatar = at(TOPBAR_AVATAR_X, 14.0, 36.0, 36.0);
    vec![
        row("top bar".into(), "topbar", 0, bar),
        row("title".into(), "title", 0, from_x(TOPBAR_TITLE_X)),
        row("search input".into(), "search-input", 0, search),
        row("export button".into(), "export", 0, export),
        row("top bar avatar".into(), "avatar", 0, avatar),
    ]
}

/// The sparkline's first bar: the bars end 20 px inside the card.
fn spark_x(card_x: f32) -> f32 {
    let span = (SPARK_BARS - 1) as f32 * SPARK_STEP + SPARK_BAR_WIDTH;
    card_x + KPI_WIDTH - 20.0 - span
}

fn card_rows(k: usize) -> Vec<Row> {
    let (x, y) = (
        SIDEBAR_WIDTH + CONTENT_PADDING + k as f32 * (KPI_WIDTH + GAP),
        TOPBAR_HEIGHT + CONTENT_PADDING,
    );
    let mut rows = vec![
        row(
            format!("card {k}"),
            "kpi",
            k,
            at(x, y, KPI_WIDTH, KPI_HEIGHT),
        ),
        row(format!("card {k} label"), "kpi-label", k, from_x(x + 20.0)),
        row(format!("card {k} value"), "kpi-value", k, from_x(x + 20.0)),
        row(
            format!("card {k} delta"),
            "delta",
            k,
            [Some(x + 20.0), Some(y + 78.0), None, Some(22.0)],
        ),
    ];
    for (i, height) in KPIS[k].spark.iter().enumerate() {
        let (bx, by) = (spark_x(x) + i as f32 * SPARK_STEP, y + 92.0 - height);
        let want = at(bx, by, SPARK_BAR_WIDTH, *height);
        rows.push(row(
            format!("card {k} bar {i}"),
            "spark-bar",
            k * SPARK_BARS + i,
            want,
        ));
    }
    rows
}

pub(super) fn rows() -> Vec<Row> {
    let content = at(
        SIDEBAR_WIDTH,
        TOPBAR_HEIGHT,
        WINDOW.0 - SIDEBAR_WIDTH,
        WINDOW.1 - TOPBAR_HEIGHT,
    );
    let mut all = bar_rows();
    all.push(row("content".into(), "content", 0, content));
    for k in 0..KPI_COUNT {
        all.extend(card_rows(k));
    }
    all
}
