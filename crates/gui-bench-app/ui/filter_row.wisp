<script>
use crate::controls::Controls;
use crate::dashboard::Status;
#[prop] let controls: Controls = Controls::unbound();
#[prop] let index: usize = 0;
let controls = controls.bound("FilterRow");
let checked: Signal<bool> = controls.shown[index];
let label = Status::ALL[index].label().to_string();
</script>

<style>
/* The row is at card y 56 + 32 k; the box is a flex item, as the user-agent sheet needs (FLAGS.md), centred on the row. */
.check-row { position: relative; display: flex; align-items: center; width: 296px; height: 32px; }
.check { flex: none; margin: 0; width: 18px; height: 18px; border-radius: 4px; --vapors-mark-color: #ffffff; }
.check:checked { background-color: #4f46e5; border-color: #4f46e5; }
.check-label { position: absolute; left: 28px; top: 0; height: 32px; display: flex; align-items: center; font-size: 14px; color: #111827; }
</style>

<div class="check-row">
  <input class="check" type="checkbox" bind:checked={checked} aria-label={label} />
  <div class="check-label">{label}</div>
</div>
