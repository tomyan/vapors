<script>
use crate::panel_row::{PanelRow, PanelRowProps};
</script>

<style>
/* A touche-style side panel built from nested components, each with its
   own <style>. This panel and its rows both use a `.title` class: the
   panel's `.panel .title` is more specific than the row's `.title`, so
   if scoping failed it would turn the rows' titles red. */
:global(html), :global(body) { height: 100%; }
:global(body) {
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #e9ecef;
}
.panel {
  width: 280px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  background-color: #ffffff;
  border: 1px solid #c7ccd1;
  border-radius: 8px;
}
.panel .title {
  padding: 8px 12px;
  color: #ffffff;
  background-color: #d32f2f;
  border-radius: 4px;
  font-size: 15px;
}
</style>

<div class="panel">
  <div class="title">Adjustments</div>
  <PanelRow label={String::from("Curves")} />
  <PanelRow label={String::from("White point")} />
  <PanelRow label={String::from("Background")} />
</div>
