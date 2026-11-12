<script>
let visible: Signal<bool> = Signal::new(true);
let locked: Signal<bool> = Signal::new(false);
</script>

<style>
/* Checkboxes (3-E3b): two bound to signals, each with a readout of its
   state. A click or Space toggles one; the mark takes the card's
   --vapors-mark-color and :checked styles the box. */
:global(html), :global(body) { height: 100%; }
:global(body) {
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #e9ecef;
  color: #171b1f;
  font-size: 15px;
}
.card {
  box-sizing: border-box;
  width: 320px;
  padding: 20px;
  background-color: #ffffff;
  border-radius: 8px;
  --vapors-mark-color: #ffffff;
}
.row { display: flex; align-items: center; height: 24px; }
input { margin: 0 12px 0 0; width: 18px; height: 18px; border-radius: 4px; }
input:checked { background-color: #3a7bd5; border-color: #3a7bd5; }
.label { width: 80px; }
.state { color: #5a6470; }
</style>

<div class="card"><div class="row"><input type="checkbox" bind:checked={visible} /><div class="label">Visible</div><div class="state">{visible}</div></div><div class="row"><input type="checkbox" bind:checked={locked} /><div class="label">Locked</div><div class="state">{locked}</div></div></div>
