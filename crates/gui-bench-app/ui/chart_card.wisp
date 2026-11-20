<script>
use crate::chart_plot::{ChartPlot, ChartPlotProps};
use crate::controls::Controls;
use crate::dashboard::{CHART_TITLE, SEGMENTS};
use crate::segment::{Segment, SegmentProps};
#[prop] let controls: Controls = Controls::new();
let title = CHART_TITLE.to_string();
let (series, compare) = (controls.series, controls.compare);
let segments: Vec<(usize, String)> = SEGMENTS.iter().enumerate().map(|(i, name)| (i, (*name).to_string())).collect();
</script>

<style>
.chart { position: relative; flex: none; width: 640px; height: 320px; box-sizing: border-box; border-radius: 12px; background-color: #ffffff; box-shadow: 0 4px 12px #1118271a; }
.chart-title { position: absolute; left: 20px; top: 20px; font-size: 16px; line-height: 20px; font-weight: 600; color: #111827; }
.segments { position: absolute; left: 424px; top: 16px; width: 196px; height: 32px; box-sizing: border-box; padding: 2px; border-radius: 8px; background-color: #f3f4f6; display: flex; }
.grid { position: absolute; left: 20px; width: 600px; height: 1px; background-color: #eef0f3; }
.g0 { top: 84px; }
.g1 { top: 138px; }
.g2 { top: 192px; }
.g3 { top: 246px; }
</style>

<div class="chart">
  <div class="chart-title">{title}</div>
  <div class="segments">
    {for seg in segments.clone() key=seg.0}<Segment index={seg.get().0} label={seg.get().1} series={series} />{/for}
  </div>
  <div class="grid g0"></div><div class="grid g1"></div><div class="grid g2"></div><div class="grid g3"></div>
  <ChartPlot series={series} compare={compare} />
</div>
