<script>
use crate::controls::Controls;
use crate::dashboard::MENU_ITEMS;
#[prop] let controls: Controls;
let menu: Signal<bool> = controls.menu;
// An item does nothing but close the menu.
let close = move || menu.set(false);
let items: Vec<(usize, String)> = MENU_ITEMS.iter().enumerate().map(|(i, label)| (i, (*label).to_string())).collect();
</script>

<style>
/* Placed in the box that holds the export button: its left edge under the button's, 4 px below its 36 px (the spec's MENU_Y is 54 for a button at y 14..50). z-index 1 paints it over the content, which comes later in the tree and holds positioned boxes. */
.menu { position: absolute; left: 0; top: 40px; z-index: 1; width: 160px; box-sizing: border-box; padding: 4px; border: 1px solid #e5e7eb; border-radius: 8px; background-color: #ffffff; box-shadow: 0 4px 12px #1118271a; }
.menu-item { height: 36px; padding: 0 12px; border-radius: 6px; display: flex; align-items: center; font-size: 14px; color: #111827; }
.menu-item:hover { background-color: #f3f4f6; }
</style>

{if menu.get()}
<div class="menu">
  {for item in items.clone() key=item.0}<div class="menu-item" onclick={close}>{item.get().1}</div>{/for}
</div>
{/if}
