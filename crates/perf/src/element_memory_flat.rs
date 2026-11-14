//! Whether a page's live heap per element stays flat (finish-line
//! property 8): the bytes each element adds, as the slope between page
//! sizes, so the costs a page pays once (fonts, the painter's pipelines
//! and atlas: 5.70 MB on a 0-row page) do not count. A structure sized
//! by the rows per element makes the slope grow with the page; the
//! slack of a list grown by doubling makes it move by less than half.

use std::fmt::Write;

use crate::element_memory::{ElementMemory, OWNERS};

/// The page sizes measured, as `--scaling`'s.
pub const SIZES: [usize; 3] = [1000, 3000, 10_000];

/// How much the total's slope may grow from 1,000 to 3,000 rows to
/// 3,000 to 10,000. Measured with the fixes of #327 and the layout
/// boxes cut to length: 10,737 then 11,223 bytes per element, 4.5%.
/// Before #327 it was 17,078 then 28,317, 66%, as a structure sized by
/// the rows per element makes it.
pub const TOTAL_GROWTH_PERCENT: i64 = 25;

/// How much one owner's slope may grow, if it also grows by more than
/// [`OWNER_GROWTH_FLOOR`]. Measured after the fixes: the mount 17%
/// (the element and signal arenas' doubling slack, bounded), style 36%
/// of under 130 bytes; paint before #327, 155% and 10 KB.
pub const OWNER_GROWTH_PERCENT: i64 = 50;

/// Bytes per element an owner's slope may grow by whatever its share.
pub const OWNER_GROWTH_FLOOR: i64 = 512;

/// Bytes per element `of` an owner (or the total, `None`) adds from
/// page `small` to page `large`.
fn slope(small: &ElementMemory, large: &ElementMemory, owner: Option<usize>) -> i64 {
    let bytes = |page: &ElementMemory| owner.map_or(page.total, |at| page.owners[at]);
    let elements = i64::try_from(large.elements - small.elements).expect("fits i64");
    (bytes(large) - bytes(small)) / elements.max(1)
}

/// Each grown slope beyond its limit, named; none when the page is flat.
///
/// # Errors
/// Pages not of [`SIZES`]' rows in that order, naming what was given.
pub fn violations(pages: &[ElementMemory]) -> Result<Vec<String>, String> {
    let rows: Vec<usize> = pages.iter().map(|page| page.rows).collect();
    let [small, middle, large] = pages else {
        return Err(format!(
            "the pages are of {rows:?} rows; the check needs {SIZES:?}"
        ));
    };
    if rows != SIZES {
        return Err(format!(
            "the pages are of {rows:?} rows; the check needs {SIZES:?}"
        ));
    }
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
    Ok(found)
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
