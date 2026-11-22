<script>
use crate::dashboard::COLUMNS;
use crate::list::{LIST_HEIGHT, keys};
use crate::order_row::{OrderRow, OrderRowProps};
use vapors_components::virtual_list::{VirtualList, VirtualListProps};
#[prop] let count: usize = 0;
// The sort state of the AMOUNT title; the rows follow it through `keys`.
let sorted: Signal<bool> = Signal::new(false);
let keys: Memo<Vec<usize>> = Memo::new(move || keys(count, sorted.get()));
let titles = COLUMNS;
let toggle = move || sorted.update(|on| *on = !*on);
</script>

<style>
:global(html), :global(body) { margin: 0; width: 100%; height: 100%; overflow: hidden; background-color: #ffffff; color: #111827; font-family: sans-serif; font-size: 14px; }
.list-head { position: relative; height: 44px; box-sizing: border-box; border-bottom: 1px solid #e5e7eb; background-color: #f9fafb; }
.col-title { position: absolute; top: 0; height: 43px; display: flex; align-items: center; font-size: 12px; font-weight: 600; color: #6b7280; }
.col-title:nth-child(1) { left: 24px; } .col-title:nth-child(2) { left: 140px; } .col-title:nth-child(3) { left: 440px; } .col-title:nth-child(4) { left: 600px; }
.sort { right: 24px; width: 120px; justify-content: flex-end; }
.sort.sorted { color: #4f46e5; }
</style>

<div class="list-head">
  <div class="col-title">{titles[0]}</div><div class="col-title">{titles[1]}</div><div class="col-title">{titles[2]}</div><div class="col-title">{titles[3]}</div>
  <div class="col-title sort" class:sorted={sorted.get()} onclick={toggle}>{titles[4]}</div>
</div>
<VirtualList keys={keys} row_height={52.0} height={LIST_HEIGHT}>{snippet row(key: usize)}<OrderRow index={key.get()} amount_at_end={true} />{/snippet}</VirtualList>
