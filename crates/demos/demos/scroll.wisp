<script>
let rows: Signal<Vec<u32>> = Signal::new((0..200).collect());
</script>

<style>
/* Scroll regions (F2c S9): two lists of 200 rows, each scrolling
   inside its own region by wheel and by finger, under a fixed bar. The
   page itself does not scroll. Rows are 40 px, alternating white and
   grey, so a scroll by three rows shows which row is at the top. */
:global(html), :global(body) { height: 100%; }
:global(html) { overflow: hidden; }
:global(body) {
  margin: 0;
  display: grid;
  grid-template-rows: 44px minmax(0, 1fr);
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  background-color: #e9ecef;
  font-size: 14px;
  color: #171b1f;
}
.bar {
  grid-column-start: 1;
  grid-column-end: 3;
  display: flex;
  align-items: center;
  padding: 0 12px;
  background-color: #171b1f;
  color: #e4e8ec;
}
.list { overflow: auto; border-right: 1px solid #c7ccd1; scrollbar-color: #3a4148 transparent; scrollbar-width: thin; }
.row { height: 40px; display: flex; align-items: center; padding: 0 12px; background-color: #ffffff; }
.row:nth-child(even) { background-color: #dde1e5; }
</style>

<div class="bar">Layers and history</div>
<div class="list">{for r in rows.get() key=r}<div class="row">Layer {r}</div>{/for}</div>
<div class="list">{for r in rows.get() key=r}<div class="row">Entry {r}</div>{/for}</div>
