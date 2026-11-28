<script>
use crate::app::order_ids;
use crate::controls::Controls;
use crate::dashboard::{COLUMNS, orders};
use crate::order_row::{OrderRow, OrderRowProps};
#[prop] let controls: Controls;
#[prop] let count: usize = 0;
// The sort state of the AMOUNT title; the filters and the search are the shared controls.
let sorted: Signal<bool> = Signal::new(false);
let base = orders(count);
let ids: Memo<Vec<usize>> = Memo::new(move || {
    let shown = controls.shown.map(|shown| shown.get());
    order_ids(&base, shown, &controls.search.get(), sorted.get())
});
let titles = COLUMNS;
let toggle = move || sorted.update(|on| *on = !*on);
</script>

<style>
.orders { position: relative; flex: none; box-sizing: border-box; overflow: hidden; border-radius: 12px; background-color: #ffffff; box-shadow: 0 4px 12px #1118271a; }
.orders-head { position: relative; height: 44px; box-sizing: border-box; border-bottom: 1px solid #e5e7eb; background-color: #f9fafb; }
.col-title { position: absolute; top: 0; height: 43px; display: flex; align-items: center; font-size: 12px; font-weight: 600; color: #6b7280; }
.col-title:nth-child(1) { left: 24px; } .col-title:nth-child(2) { left: 140px; } .col-title:nth-child(3) { left: 440px; } .col-title:nth-child(4) { left: 600px; }
.sort { left: 848px; width: 120px; justify-content: flex-end; }
.sort.sorted { color: #4f46e5; }
</style>

<div class="orders">
  <div class="orders-head">
    <div class="col-title">{titles[0]}</div><div class="col-title">{titles[1]}</div><div class="col-title">{titles[2]}</div><div class="col-title">{titles[3]}</div>
    <div class="col-title sort" class:sorted={sorted.get()} onclick={toggle}>{titles[4]}</div>
  </div>
  {for r in ids.get() key=r}<OrderRow index={r.get()} />{/for}
</div>
