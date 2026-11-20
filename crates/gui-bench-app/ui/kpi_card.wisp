<script>
use crate::app::spark_heights;
use crate::controls::Controls;
use crate::dashboard::kpi_value;
#[prop] let index: usize = 0;
#[prop] let label: String = String::new();
#[prop] let delta: String = String::new();
#[prop] let up: bool = false;
#[prop] let controls: Controls;
// 0 until the live switch is on.
let tick = controls.live.tick;
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
</style>

<div class="kpi">
  <div class="kpi-label">{label}</div>
  <div class="kpi-value">{kpi_value(index, tick.get())}</div>
  <div class="delta" class:up={up}>{delta}</div>
  <div class="spark">
    {for bar in bars.clone() key=bar.0}<div class="spark-bar" style:height={bar.get().1}></div>{/for}
  </div>
</div>
