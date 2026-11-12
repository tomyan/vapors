<script>
let amount: Signal<f64> = Signal::new(40.0);
let exposure: Signal<f64> = Signal::new(0.0);
</script>

<style>
/* Number fields (3-E3c): one bound to a signal, which takes the value
   typed once Enter or a click elsewhere commits it, with its readout;
   and a scrubbable one (data-scrub), dragged sideways to step. A
   field :invalid while its text is no number or out of range shows a
   red border. */
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
.row + .row { margin-top: 8px; }
.label { width: 80px; }
input { width: 100px; height: 24px; }
input:invalid { border-color: #d03030; }
.value { width: 60px; text-align: right; color: #5a6470; }
</style>

<div class="card"><div class="row"><div class="label">Amount</div><input type="number" min="0" max="100" step="1" bind:value={amount} /><div class="value">{amount}</div></div><div class="row"><div class="label">Exposure</div><input type="number" min="-5" max="5" step="0.1" data-scrub="4" bind:value={exposure} /><div class="value">{exposure}</div></div></div>
