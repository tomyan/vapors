<script>
use crate::dashboard::{AVATAR_COLORS, order};
#[prop] let index: usize = 0;
let found = order(index);
let (number, name, initials, amount) = (found.number_text(), found.customer.clone(), found.initials.clone(), found.amount());
let colour = AVATAR_COLORS.iter().position(|c| *c == found.avatar).expect("avatar colour not in AVATAR_COLORS");
let status = found.status.filter_row();
assert!(status < 4, "status {status} has no pill class: add s{status} to order_row.wisp (cargo test -p gui-bench-app)");
let label = found.status.label().to_string();
let progress = found.progress;
assert!(progress <= 100, "progress {progress} is not a percentage: the fill has 7 bit segments, 0..=100 only (cargo test -p gui-bench-app)");
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
.c0 { background-color: #f87171; } .c1 { background-color: #fb923c; } .c2 { background-color: #fbbf24; } .c3 { background-color: #34d399; }
.c4 { background-color: #22d3ee; } .c5 { background-color: #60a5fa; } .c6 { background-color: #a78bfa; } .c7 { background-color: #f472b6; }
.pill { height: 22px; padding: 0 10px; border-radius: 11px; font-size: 12px; font-weight: 600; display: flex; align-items: center; }
.s0 { background-color: #dcfce7; color: #166534; } .s1 { background-color: #fef3c7; color: #92400e; }
.s2 { background-color: #dbeafe; color: #1e40af; } .s3 { background-color: #fee2e2; color: #991b1b; }
.track { position: relative; width: 120px; height: 6px; border-radius: 3px; background-color: #e5e7eb; }
/* The fill is `progress` * 1.2 px: seven segments, one per bit of the percentage (FLAGS.md). */
.fill { position: absolute; left: 0; top: 0; height: 6px; display: flex; border-radius: 3px; background-color: #4f46e5; }
.b0, .b1, .b2, .b3, .b4, .b5, .b6 { height: 6px; width: 0; }
.b0.on { width: 1.2px; } .b1.on { width: 2.4px; } .b2.on { width: 4.8px; } .b3.on { width: 9.6px; }
.b4.on { width: 19.2px; } .b5.on { width: 38.4px; } .b6.on { width: 76.8px; }
</style>

<div class="order-row">
  <div class="cell c-number"><span class="order-number">{number}</span></div>
  <div class="cell c-customer">
    <div class="order-avatar" class:c0={colour == 0} class:c1={colour == 1} class:c2={colour == 2} class:c3={colour == 3} class:c4={colour == 4} class:c5={colour == 5} class:c6={colour == 6} class:c7={colour == 7}>{initials}</div>
    <span class="order-name">{name}</span>
  </div>
  <div class="cell c-status"><div class="pill" class:s0={status == 0} class:s1={status == 1} class:s2={status == 2} class:s3={status == 3}>{label}</div></div>
  <div class="cell c-progress"><div class="track"><div class="fill">
    <div class="b0" class:on={progress & 1 != 0}></div><div class="b1" class:on={progress & 2 != 0}></div><div class="b2" class:on={progress & 4 != 0}></div><div class="b3" class:on={progress & 8 != 0}></div><div class="b4" class:on={progress & 16 != 0}></div><div class="b5" class:on={progress & 32 != 0}></div><div class="b6" class:on={progress & 64 != 0}></div>
  </div></div></div>
  <div class="cell c-amount"><span class="order-amount">{amount}</span></div>
</div>
