<script>
use crate::sidebar::{Sidebar, SidebarProps};
// The active nav item; the first at startup.
let active: Signal<usize> = Signal::new(0);
</script>

<style>
:global(html), :global(body) { margin: 0; width: 100%; height: 100%; overflow: hidden; background-color: #f3f4f6; color: #111827; font-family: sans-serif; font-size: 14px; }
.app { display: flex; width: 100%; height: 100%; }
</style>

<div class="app">
  <Sidebar active={active} />
</div>
