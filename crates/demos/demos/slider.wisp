<script>
let level: Signal<f64> = Signal::new(40.0);
</script>

<style>
/* A range input (3-E3a): a card with a slider bound to a signal and a
   readout of it. Its thumb and track take the card's --vapors-* parts;
   a press anywhere on the track moves the thumb there, a drag follows
   the pointer, and the arrow keys step it once focused (Shift by ten). */
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
  --vapors-thumb-color: #3a7bd5;
  --vapors-track-color: #c7ccd1;
}
.row { display: flex; align-items: center; }
input { width: 200px; height: 24px; margin: 0; }
.value { width: 60px; text-align: right; }
</style>

<div class="card"><div class="row"><input type="range" min="0" max="100" step="1" bind:value={level} /><div class="value">{level}</div></div></div>
