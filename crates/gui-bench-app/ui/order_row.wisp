<script>
use crate::dashboard::order;
use crate::data_colours::{avatar, pill_background, pill_text};
use crate::order_parts::{fill_width, parts};
#[prop] let index: usize = 0;
let found = order(index);
let (number, name, initials, amount) = (found.number_text(), found.customer.clone(), found.initials.clone(), found.amount());
// Refuses an order the CSS below has no class for (order_parts.rs).
let (colour, status, progress) = parts(&found);
let fill_width = fill_width(progress);
let (avatar_back, pill_back, pill_fore) = (avatar(colour), pill_background(status), pill_text(status));
let label = found.status.label().to_string();
</script>

<style>
.order-row { position: relative; height: 52px; box-sizing: border-box; border-bottom: 1px solid #f3f4f6; }
.order-row:hover { background-color: #eef2ff; }
.cell { position: absolute; top: 0; height: 51px; display: flex; align-items: center; }
.c-number { left: 24px; font-size: 14px; color: #6b7280; }
.c-customer { left: 140px; gap: 12px; font-size: 14px; color: #111827; }
.c-status { left: 440px; }
.c-progress { left: 600px; }
.c-amount { left: 848px; width: 120px; justify-content: flex-end; font-size: 14px; font-weight: 600; color: #111827; }
.order-avatar { flex: none; width: 28px; height: 28px; border-radius: 14px; color: #ffffff; font-size: 11px; font-weight: 600; display: flex; align-items: center; justify-content: center; }
.pill { height: 22px; padding: 0 10px; border-radius: 11px; font-size: 12px; font-weight: 600; display: flex; align-items: center; }
.track { position: relative; width: 120px; height: 6px; border-radius: 3px; background-color: #e5e7eb; }
/* The fill is `progress` * 1.2 px wide, bound by style:width (FLAGS.md). */
.fill { position: absolute; left: 0; top: 0; height: 6px; border-radius: 3px; background-color: #4f46e5; }
</style>

<div class="order-row">
  <div class="cell c-number"><span class="order-number">{number}</span></div>
  <div class="cell c-customer">
    <div class="order-avatar" style:background-color={avatar_back}>{initials}</div>
    <span class="order-name">{name}</span>
  </div>
  <div class="cell c-status"><div class="pill" style:background-color={pill_back} style:color={pill_fore}>{label}</div></div>
  <div class="cell c-progress"><div class="track"><div class="fill" style:width={fill_width}></div></div></div>
  <div class="cell c-amount"><span class="order-amount">{amount}</span></div>
</div>
