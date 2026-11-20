<script>
use crate::app::spark_heights;
use crate::dashboard::kpi_value;
#[prop] let index: usize = 0;
#[prop] let label: String = String::new();
#[prop] let delta: String = String::new();
#[prop] let up: bool = false;
// The tick is 0 until s3's live switch runs.
let tick: Signal<u64> = Signal::new(0);
let bars = spark_heights(index);
</script>

<style>
.kpi { position: relative; flex: 1; min-width: 0; height: 112px; box-sizing: border-box; border-radius: 12px; background-color: #ffffff; box-shadow: 0 4px 12px #1118271a; }
.kpi-label { position: absolute; left: 20px; top: 18px; font-size: 13px; line-height: 16px; color: #6b7280; }
.kpi-value { position: absolute; left: 20px; top: 40px; font-size: 24px; line-height: 30px; font-weight: 700; color: #111827; }
.delta { position: absolute; left: 20px; top: 78px; height: 22px; padding: 0 8px; border-radius: 11px; font-size: 12px; font-weight: 600; display: flex; align-items: center; background-color: #fee2e2; color: #991b1b; }
.delta.up { background-color: #dcfce7; color: #166534; }
.spark { position: absolute; right: 20px; top: 0; height: 92px; display: flex; align-items: flex-end; gap: 3px; }
.spark-bar { width: 6px; border-radius: 2px; background-color: #c7d2fe; }
.h14 { height: 14px; }
.h16 { height: 16px; }
.h18 { height: 18px; }
.h20 { height: 20px; }
.h22 { height: 22px; }
.h24 { height: 24px; }
.h26 { height: 26px; }
.h28 { height: 28px; }
.h30 { height: 30px; }
.h32 { height: 32px; }
.h34 { height: 34px; }
.h36 { height: 36px; }
.h38 { height: 38px; }
.h40 { height: 40px; }
.h42 { height: 42px; }
.h44 { height: 44px; }
</style>

<div class="kpi">
  <div class="kpi-label">{label}</div>
  <div class="kpi-value">{kpi_value(index, tick.get())}</div>
  <div class="delta" class:up={up}>{delta}</div>
  <div class="spark">
    {for bar in bars.clone() key=bar.0}<div class="spark-bar" class:h14={bar.get().1 == 14} class:h16={bar.get().1 == 16} class:h18={bar.get().1 == 18} class:h20={bar.get().1 == 20} class:h22={bar.get().1 == 22} class:h24={bar.get().1 == 24} class:h26={bar.get().1 == 26} class:h28={bar.get().1 == 28} class:h30={bar.get().1 == 30} class:h32={bar.get().1 == 32} class:h34={bar.get().1 == 34} class:h36={bar.get().1 == 36} class:h38={bar.get().1 == 38} class:h40={bar.get().1 == 40} class:h42={bar.get().1 == 42} class:h44={bar.get().1 == 44}></div>{/for}
  </div>
</div>
