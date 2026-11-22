<script>
use crate::chart_bars::{current_height, previous_height};
use crate::dashboard::CHART_BARS;
#[prop] let series: Signal<usize> = Signal::new(0);
#[prop] let compare: Signal<bool> = Signal::new(false);
let slots: Vec<usize> = (0..CHART_BARS).collect();
</script>

<style>
/* Bar heights are bound: style:height of the spec's `bar_height` and `previous_height`
   at the series and slot (chart_bars.rs, FLAGS.md); a test holds every bar to Chromium's. */
.plot { position: absolute; left: 20px; top: 72px; width: 600px; height: 228px; display: flex; }
.slot { position: relative; flex: none; width: 25px; height: 228px; }
.cur { position: absolute; bottom: 0; left: 4px; width: 17px; border-radius: 4px 4px 0 0; background-color: #6366f1; }
.prev { position: absolute; bottom: 0; left: 3px; width: 8px; border-radius: 4px 4px 0 0; background-color: #c7d2fe; }
.cmp .cur { left: 13px; width: 8px; }
</style>

<div class="plot" class:cmp={compare.get()}>
  {for slot in slots.clone() key=slot}<div class="slot" data-slot={slot.get().to_string()}>{if compare.get()}<div class="prev" style:height={previous_height(series.get(), slot.get())}></div>{/if}<div class="cur" style:height={current_height(series.get(), slot.get())}></div></div>{/for}
</div>
