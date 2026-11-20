<script>
use crate::sidebar::{Sidebar, SidebarProps};
use crate::topbar::{Topbar, TopbarProps};
use crate::content::{Content, ContentProps};
// The active nav item, the first at startup; the search text, which the
// orders (s2) filter by.
let active: Signal<usize> = Signal::new(0);
let search: Signal<String> = Signal::new(String::new());
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
    <Content />
  </div>
</div>
