<script>
let tick: Signal<u32> = Signal::new(0);
let shown: Signal<String> = Signal::new(String::from("measuring"));
let rows: Signal<Vec<u32>> = Signal::new((0..100).collect());
let cells: Signal<Vec<u32>> = Signal::new((0..9).collect());
</script>

<style>
/* 1,000 elements (100 rows of a row and 9 cells) under a counter that
   ticks every frame, and the time the last Document::update took, as
   measured by the app (F2c S7). The budget is 4 ms (docs/F2c.md). */
:global(html), :global(body) { height: 100%; }
:global(body) { margin: 0; background-color: #e9ecef; font-size: 11px; color: #171b1f; }
.top { display: flex; gap: 12px; padding: 8px; background-color: #171b1f; color: #ffffff; font-size: 14px; }
.tick { width: 120px; }
.row { display: flex; gap: 2px; padding: 0 8px; }
.cell { width: 38px; background-color: #ffffff; }
</style>

<div class="top"><div class="tick">tick {tick}</div><div class="time">update {shown}</div></div>
{for r in rows.get() key=r}<div class="row">{for c in cells.get() key=c}<div class="cell">{r}.{c}</div>{/for}</div>{/for}
