<script>
use crate::sidebar::{Sidebar, SidebarProps};
use crate::topbar::{Topbar, TopbarProps};
use crate::content::{Content, ContentProps};
use crate::controls::Controls;
// The active nav item, the first at startup; the controls the chart, the
// filters and the search input share; the count of messages the feed shows.
let active: Signal<usize> = Signal::new(0);
#[prop] let rows: usize = 0;
#[prop] let controls: Controls;
let search: Signal<String> = controls.search;
</script>

<style>
:global(html), :global(body) { margin: 0; width: 100%; height: 100%; overflow: hidden; background-color: #f3f4f6; color: #111827; font-family: sans-serif; font-size: 14px; }
.app { display: flex; width: 100%; height: 100%; }
.main { position: relative; flex: 1; min-width: 0; display: flex; flex-direction: column; height: 100%; }
</style>

<div class="app">
  <Sidebar active={active} />
  <div class="main">
    <Topbar search={search} controls={controls} />
    <Content controls={controls} active={active} rows={rows} />
  </div>
</div>
