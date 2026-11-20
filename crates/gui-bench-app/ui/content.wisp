<script>
use crate::dashboard::{KPI_COUNT, KPIS};
use crate::chart_card::{ChartCard, ChartCardProps};
use crate::controls::Controls;
use crate::filter_card::{FilterCard, FilterCardProps};
use crate::kpi_card::{KpiCard, KpiCardProps};
#[prop] let controls: Controls = Controls::new();
let cards: Vec<usize> = (0..KPI_COUNT).collect();
</script>

<style>
/* The scroll container with the framework's defaults: padding 24, 16 between blocks. */
.content { flex: 1; min-height: 0; box-sizing: border-box; overflow: auto; padding: 24px; display: flex; flex-direction: column; gap: 16px; }
.kpis { flex: none; display: flex; gap: 16px; }
.row2 { flex: none; display: flex; gap: 16px; }
</style>

<div class="content">
  <div class="kpis">
    {for k in cards.clone() key=k}<KpiCard index={k.get()} label={KPIS[k.get()].label.to_string()} delta={KPIS[k.get()].delta.to_string()} up={KPIS[k.get()].up} />{/for}
  </div>
  <div class="row2">
    <ChartCard controls={controls} />
    <FilterCard controls={controls} />
  </div>
</div>
