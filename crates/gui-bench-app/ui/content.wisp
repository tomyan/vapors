<script>
use crate::dashboard::{KPI_COUNT, KPIS, Page, page};
use crate::chart_card::{ChartCard, ChartCardProps};
use crate::controls::Controls;
use crate::empty_page::{EmptyPage, EmptyPageProps};
use crate::filter_card::{FilterCard, FilterCardProps};
use crate::kpi_card::{KpiCard, KpiCardProps};
use crate::message_card::{MessageCard, MessageCardProps};
use crate::orders_card::{OrdersCard, OrdersCardProps};
#[prop] let controls: Controls;
#[prop] let active: Signal<usize> = Signal::new(0);
#[prop] let rows: usize = 0;
let cards: Vec<usize> = (0..KPI_COUNT).collect();
let feed: Vec<usize> = (0..rows).collect();
</script>

<style>
/* The scroll container with the framework's defaults: padding 24, 16 between blocks (12 between messages). */
.content { flex: 1; min-height: 0; box-sizing: border-box; overflow-y: auto; padding: 24px; display: flex; flex-direction: column; gap: 16px; }
.kpis { flex: none; display: flex; gap: 16px; }
.row2 { flex: none; display: flex; gap: 16px; }
.feed { flex: none; display: flex; flex-direction: column; gap: 12px; }
</style>

<div class="content">
  {if page(active.get()) == Page::Overview}
  <div class="kpis">
    {for k in cards.clone() key=k}<KpiCard index={k.get()} label={KPIS[k.get()].label.to_string()} delta={KPIS[k.get()].delta.to_string()} up={KPIS[k.get()].up} />{/for}
  </div>
  <div class="row2">
    <ChartCard controls={controls} />
    <FilterCard controls={controls} />
  </div>
  <OrdersCard controls={controls} count={rows} />
  {else}
  {if page(active.get()) == Page::Messages}
  <div class="feed">
    {for m in feed.clone() key=m}<MessageCard index={m.get()} />{/for}
  </div>
  {else}
  <EmptyPage />
  {/if}
  {/if}
</div>
