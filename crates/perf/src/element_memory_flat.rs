//! Whether a page's live heap per element stays flat and bounded
//! (finish-line property 8): the bytes each element adds, as the slope
//! between page sizes, so the costs a page pays once (fonts, the
//! painter's pipelines and atlas: 5.70 MB on a 0-row page) do not count.
//! The sizes go by fours, so every list grown by doubling is as full at
//! each (its slope is then constant): a structure sized by the rows per
//! element makes the slope grow, and doubling slack does not. Property
//! 8's absolute part is held too: one style per distinct style, the same
//! count at every size, and the total per element within a multiple of
//! the per-box layout records'.

use std::fmt::Write;

use crate::element_memory::{ElementMemory, OWNERS};

/// The page sizes measured: rows by fours, not `--scaling`'s 1,000,
/// 3,000 and 10,000, so doubling lists sit at one fill at every size
/// (review of #333, M1). Each row is four elements.
pub const SIZES: [usize; 3] = [1024, 4096, 16_384];

/// How much the total's slope may grow from the first pair of sizes to
/// the second. Measured with #327 and the layout boxes cut to length:
/// 11,337 then 11,384 bytes per element, 0.4%. Before #327 the slope
/// grew 66% at 1,000/3,000/10,000 rows.
pub const TOTAL_GROWTH_PERCENT: i64 = 2;

/// How much one owner's slope may grow, if it also grows by more than
/// [`OWNER_GROWTH_FLOOR`]. Measured: layout 0.7% (6,814 then 6,859), the
/// mount, style and paint not at all.
pub const OWNER_GROWTH_PERCENT: i64 = 5;

/// Bytes per element an owner's slope may grow by whatever its share.
pub const OWNER_GROWTH_FLOOR: i64 = 64;

/// The most the total per element may be, as a multiple of layout's:
/// property 8 bounds an element's memory by its layout records.
/// Measured 11,384 against 6,859, 1.66 times.
pub const TOTAL_OVER_LAYOUT: i64 = 2;

/// Bytes per element `of` an owner (or the total, `None`) adds from
/// page `small` to page `large`.
fn slope(small: &ElementMemory, large: &ElementMemory, owner: Option<usize>) -> i64 {
    let bytes = |page: &ElementMemory| owner.map_or(page.total, |at| page.owners[at]);
    let elements = i64::try_from(large.elements - small.elements).expect("fits i64");
    (bytes(large) - bytes(small)) / elements.max(1)
}

/// Each limit the pages break, named; none when property 8 holds.
///
/// # Errors
/// Pages not of [`SIZES`]' rows in that order, naming what was given.
pub fn violations(pages: &[ElementMemory]) -> Result<Vec<String>, String> {
    let rows: Vec<usize> = pages.iter().map(|page| page.rows).collect();
    let (true, [small, middle, large]) = (rows == SIZES, pages) else {
        return Err(format!(
            "the pages are of {rows:?} rows; the check needs {SIZES:?}"
        ));
    };
    let mut found = grown_slopes([small, middle, large]);
    found.extend(styles_not_shared(pages));
    found.extend(over_layout(middle, large));
    Ok(found)
}

/// The total's and each owner's slope that grew past its limit.
fn grown_slopes([small, middle, large]: [&ElementMemory; 3]) -> Vec<String> {
    let grown = |owner| (slope(small, middle, owner), slope(middle, large, owner));
    let mut found = Vec::new();
    let (before, after) = grown(None);
    if after * 100 > before * (100 + TOTAL_GROWTH_PERCENT) {
        found.push(format!(
            "total: {before} then {after} bytes per element, over {TOTAL_GROWTH_PERCENT}% more"
        ));
    }
    for (at, owner) in OWNERS.iter().enumerate() {
        let (before, after) = grown(Some(at));
        let over_share = after * 100 > before.max(0) * (100 + OWNER_GROWTH_PERCENT);
        if over_share && after - before > OWNER_GROWTH_FLOOR {
            found.push(format!(
                "{owner}: {before} then {after} bytes per element, over \
                 {OWNER_GROWTH_PERCENT}% and {OWNER_GROWTH_FLOOR} bytes more"
            ));
        }
    }
    found
}

/// The styles computed, refused when they differ between sizes: one
/// style per distinct style, not one per element.
fn styles_not_shared(pages: &[ElementMemory]) -> Option<String> {
    let styles: Vec<usize> = pages.iter().map(|page| page.styles_computed).collect();
    let first = styles[0];
    styles.iter().any(|&count| count != first).then(|| {
        format!(
            "styles computed {styles:?} at {SIZES:?} rows: one per distinct style, not per element"
        )
    })
}

/// The total per element over [`TOTAL_OVER_LAYOUT`] times layout's.
fn over_layout(middle: &ElementMemory, large: &ElementMemory) -> Option<String> {
    let layout = OWNERS
        .iter()
        .position(|owner| *owner == "layout")
        .expect("an owner");
    let (total, boxes) = (
        slope(middle, large, None),
        slope(middle, large, Some(layout)),
    );
    (total > boxes * TOTAL_OVER_LAYOUT).then(|| {
        format!("total: {total} bytes per element, over {TOTAL_OVER_LAYOUT} times layout's {boxes}")
    })
}

/// The pages' table: elements, styles computed, and per element (the
/// slope from the page before) for the total and each owner.
#[must_use]
pub fn table(pages: &[ElementMemory]) -> String {
    let mut out = format!(
        "| rows | elements | styles computed | total MB | total B/element | {} |\n|{}\n",
        OWNERS.map(|owner| format!("{owner} B/element")).join(" | "),
        "---|".repeat(5 + OWNERS.len())
    );
    for (at, page) in pages.iter().enumerate() {
        let per = |owner| {
            at.checked_sub(1).map_or("-".into(), |prev| {
                slope(&pages[prev], page, owner).to_string()
            })
        };
        let owners: Vec<String> = (0..OWNERS.len()).map(|o| per(Some(o))).collect();
        #[allow(clippy::cast_precision_loss)] // bytes shown in MB
        let mb = page.total as f64 / 1e6;
        writeln!(
            out,
            "| {} | {} | {} | {mb:.1} | {} | {} |",
            page.rows,
            page.elements,
            page.styles_computed,
            per(None),
            owners.join(" | ")
        )
        .expect("a String takes any write");
    }
    out
}

#[cfg(test)]
#[path = "element_memory_flat_tests.rs"]
mod tests;
