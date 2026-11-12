<script>
let name: Signal<String> = Signal::new(String::from("Warm"));
</script>

<style>
/* A text field (E6a): bound to a signal with a readout of it. A click
   puts the caret where it lands, a drag selects, Shift with the arrows
   extends, Control moves by word, typing replaces the selection, and
   Tab into it selects the text whole. */
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
}
.row { display: flex; align-items: center; height: 24px; }
.label { width: 80px; }
input { width: 120px; height: 24px; }
.value { width: 80px; text-align: right; color: #5a6470; }
</style>

<div class="card"><div class="row"><div class="label">Name</div><input type="text" bind:value={name} /><div class="value">{name}</div></div></div>
