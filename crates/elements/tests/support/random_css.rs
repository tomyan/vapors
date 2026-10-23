//! The stylesheet of `incremental_random.rs` and the classes its
//! changes set.

/// Inherited properties, `var()`, `+` and `~`, `:nth-child`,
/// `:last-child`, `:empty`, an ancestor attribute, `:hover` on a
/// non-subject compound, `:active`, `@media`, calc with % and em, vw,
/// relative and absolute positioning, and z-index on positioned boxes
/// and on flex items, and `overflow: hidden`.
pub const CSS: &str = "
:root { --pad: 3px; }
body { margin: 0; font-size: 14px; color: #222222; }
.row { display: flex; gap: 4px; }
.box { padding: var(--pad); background-color: #eeeeff; }
.big { font-size: 1.2em; }
.box + .box { margin-top: 2px; }
.box ~ .alt { color: #aa0000; }
.box:nth-child(odd) { background-color: #ffffee; }
.box:last-child { border-bottom: 1px solid #333333; }
.box:empty { height: 6px; }
[data-on] .box { color: #006600; }
.row:hover .box { background-color: #ccffcc; }
.box:active { background-color: #ffcccc; }
.wide { width: calc(50% - 1em); }
.vw { width: 30vw; }
@media (min-width: 500px) { .box { padding: 6px; } }
.pos { position: relative; top: 2px; left: 1px; }
.abs { position: absolute; right: 0; top: 0; width: 20px; height: 5px; }
.up { z-index: 2; }
.down { z-index: -1; }
.cut { overflow: hidden; height: 12px; }
";

pub const CLASSES: &[&str] = &[
    "box",
    "box big",
    "box alt",
    "box wide",
    "box vw",
    "box pos",
    "box abs",
    "box abs up",
    "box pos down",
    "box up",
    "box cut",
    "row cut",
    "row",
    "",
];
