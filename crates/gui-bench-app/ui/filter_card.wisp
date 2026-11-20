<script>
use crate::controls::Controls;
use crate::dashboard::{APPLY_LABEL, COMPARE_LABEL, FILTERS_TITLE, RESET_LABEL};
use crate::filter_row::{FilterRow, FilterRowProps};
#[prop] let controls: Controls = Controls::unbound();
let controls = controls.bound("FilterCard");
let (title, compare_label, reset_label, apply_label) = (FILTERS_TITLE, COMPARE_LABEL, RESET_LABEL, APPLY_LABEL);
let compare = controls.compare;
let rows: Vec<usize> = (0..4).collect();
let toggle = move || compare.update(|on| *on = !*on);
let reset = move || controls.reset();
// Deliberately nothing: the spec's Apply changes no state (the controls act at once), and
// `apply_changes_nothing` pins that. This is not an unfinished handler.
let apply = move || ();
</script>

<style>
.filters { position: relative; flex: none; width: 336px; height: 320px; box-sizing: border-box; border-radius: 12px; background-color: #ffffff; box-shadow: 0 4px 12px #1118271a; }
.filters-title { position: absolute; left: 20px; top: 20px; font-size: 16px; line-height: 20px; font-weight: 600; color: #111827; }
.checks { position: absolute; left: 20px; top: 56px; width: 296px; }
.compare-label { position: absolute; left: 20px; top: 196px; height: 28px; display: flex; align-items: center; font-size: 14px; color: #111827; }
.switch { position: absolute; left: 276px; top: 199px; width: 40px; height: 22px; border-radius: 11px; background-color: #d1d5db; }
.switch.on { background-color: #4f46e5; }
.knob { position: absolute; left: 2px; top: 2px; width: 18px; height: 18px; border-radius: 9px; background-color: #ffffff; box-shadow: 0 1px 3px #11182740; }
.on .knob { left: 20px; }
.reset, .apply { position: absolute; top: 264px; width: 100px; height: 36px; box-sizing: border-box; margin: 0; padding: 0; border-radius: 8px; font-size: 14px; font-weight: 600; display: flex; align-items: center; justify-content: center; }
.reset { left: 104px; border: 1px solid #d1d5db; background-color: #ffffff; color: #111827; }
.apply { left: 216px; border: none; background-color: #4f46e5; color: #ffffff; box-shadow: 0 1px 3px #11182740; }
</style>

<div class="filters">
  <div class="filters-title">{title}</div>
  <div class="checks">
    {for k in rows.clone() key=k}<FilterRow controls={controls} index={k.get()} />{/for}
  </div>
  <div class="compare-label">{compare_label}</div>
  <div class="switch" class:on={compare.get()} onclick={toggle}><div class="knob"></div></div>
  <button class="reset" onclick={reset}>{reset_label}</button>
  <button class="apply" onclick={apply}>{apply_label}</button>
</div>
