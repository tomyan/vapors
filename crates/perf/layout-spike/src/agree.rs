//! Every option lays a workload out to the same boxes, before and after
//! the leaf change, against boxes worked out from `spec.rs` alone (not
//! from any option's code). The bench runs `check_all` on every workload
//! it times before it times one; `tests/agree.rs` runs it on those and more.

use crate::spec::{CHANGED_LEAF, CHANGED_W, COLS, DEEP_SIDE, LEAF, PAD, ROW_H, row_y};
use crate::{Engine, Rect, Unsupported, Workload};

type Boxes = [Rect; 2];

const fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, w, h }
}

/// The probe boxes before and after the change, by the spec's words.
///
/// # Panics
/// A deep tree under 2 levels, or a table over 65535 rows.
#[must_use]
pub fn expected(workload: Workload) -> (Boxes, Boxes) {
    match workload {
        Workload::Table(rows) => {
            let y = row_y(rows / 2);
            let before = [r(80.0, y, COLS[1], ROW_H), r(440.0, y, 560.0, ROW_H)];
            let after = [r(80.0, y, CHANGED_W, ROW_H), r(470.0, y, 530.0, ROW_H)];
            (before, after)
        }
        Workload::Deep(depth) => (deep(depth, LEAF), deep(depth, CHANGED_LEAF)),
    }
}

/// A level's box in f64: left, top, width, height.
type Level = [f64; 4];

/// A px value of the spec (integers far under 2^24) as the f32 boxes hold.
#[allow(clippy::cast_possible_truncation)]
const fn narrow(px: f64) -> f32 {
    px as f32
}

/// The leaf of a level in `level`'s box, `side` square, and the box of
/// the level below: its content box (`PAD` off each side) less the leaf,
/// below it on even levels (columns), beside it on odd ones.
fn step(level: usize, own: Level, side: f64) -> (Rect, Level) {
    let pad = f64::from(PAD);
    let [left, top, wide, high] = [
        own[0] + pad,
        own[1] + pad,
        own[2] - 2.0 * pad,
        own[3] - 2.0 * pad,
    ];
    let leaf = r(narrow(left), narrow(top), narrow(side), narrow(side));
    let below = if level.is_multiple_of(2) {
        [left, top + side, wide, high - side]
    } else {
        [left + side, top, wide - side, high]
    };
    (leaf, below)
}

/// The deep tree's probes with level `depth - 2`'s leaf `side` square,
/// worked down the levels in f64.
fn deep(depth: usize, side: f32) -> Boxes {
    assert!(depth >= 2, "a deep tree of 2 levels or more");
    let mut own = [0.0, 0.0, f64::from(DEEP_SIDE), f64::from(DEEP_SIDE)];
    let mut leaves = vec![];
    for level in 0..depth {
        let s = f64::from(if level == depth - 2 { side } else { LEAF });
        let (leaf, below) = step(level, own, s);
        leaves.push(leaf);
        own = below;
    }
    [leaves[depth - 2], leaves[depth - 1]]
}

fn close(got: Boxes, want: Boxes) -> bool {
    got.iter().zip(&want).all(|(g, w)| {
        [g.x - w.x, g.y - w.y, g.w - w.w, g.h - w.h]
            .iter()
            .all(|d| d.abs() < 1e-3)
    })
}

/// `E` on `workload`: the boxes before and after one change, or the
/// change refused where `after` is `None`.
///
/// # Errors
/// What differs, naming the option, workload and stage.
pub fn check<E: Engine>(
    workload: Workload,
    before: Boxes,
    after: Option<Boxes>,
) -> Result<(), String> {
    let what = format!("{} on {workload:?}", E::NAME);
    let mut engine = E::build(workload).map_err(|e| format!("{what}: {e:?}"))?;
    engine.layout();
    if !close(engine.probe(), before) {
        return Err(format!(
            "{what}, first layout: got {:?}, want {before:?}",
            engine.probe()
        ));
    }
    match (engine.change_leaf(), after) {
        (Ok(()), Some(after)) => {
            engine.layout();
            if close(engine.probe(), after) {
                return Ok(());
            }
            Err(format!(
                "{what}, after the change: got {:?}, want {after:?}",
                engine.probe()
            ))
        }
        (Err(Unsupported(_)), None) => Ok(()),
        (got, want) => Err(format!(
            "{what}: change gave {got:?}, wanted boxes {want:?}"
        )),
    }
}

/// Every option that takes `workload` against the spec's boxes, and the
/// boundary refusing what it cannot take, as the bench times them.
///
/// # Errors
/// The first option whose boxes differ; the fix is in that option, then
/// reproduce: cargo test --offline (in crates/perf/layout-spike).
pub fn check_all(workload: Workload) -> Result<(), String> {
    let (before, after) = expected(workload);
    check::<crate::taffy_engine::Taffy>(workload, before, Some(after))?;
    check::<crate::lanes::Lanes>(workload, before, Some(after))?;
    check::<crate::generated::Generated>(workload, before, Some(after))?;
    match workload {
        Workload::Table(_) => check::<crate::fixed::Fixed>(workload, before, None),
        Workload::Deep(_) => match crate::fixed::Fixed::build(workload) {
            Err(_) => Ok(()),
            Ok(_) => Err(format!(
                "compile-time boundary took {workload:?}, which is not static"
            )),
        },
    }
}
