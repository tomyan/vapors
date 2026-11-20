<script>
use crate::sidebar::{Sidebar, SidebarProps};
use crate::topbar::{Topbar, TopbarProps};
use crate::content::{Content, ContentProps};
use crate::controls::Controls;
// The active nav item, the first at startup; the controls the chart, the
// filters, the search input and the orders share; the orders' row count.
let active: Signal<usize> = Signal::new(0);
#[prop] let controls: Controls = Controls::new();
let search: Signal<String> = controls.search;
</script>

<style>
:global(html), :global(body) { margin: 0; width: 100%; height: 100%; overflow: hidden; background-color: #f3f4f6; color: #111827; font-family: sans-serif; font-size: 14px; }
.app { display: flex; width: 100%; height: 100%; }
.main { flex: 1; min-width: 0; display: flex; flex-direction: column; height: 100%; }
</style>

<div class="app">
  <Sidebar active={active} />
  <div class="main">
    <Topbar search={search} />
    <Content controls={controls} />
  </div>
</div>
