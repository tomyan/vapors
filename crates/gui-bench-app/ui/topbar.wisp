<script>
use crate::controls::Controls;
use crate::export_menu::{ExportMenu, ExportMenuProps};
use crate::dashboard::{EXPORT_LABEL, LIVE_LABEL, PAGE_TITLE, SEARCH_PLACEHOLDER, USER_INITIALS};
let (title, placeholder, export, initials, live_label) = (PAGE_TITLE, SEARCH_PLACEHOLDER, EXPORT_LABEL, USER_INITIALS, LIVE_LABEL);
#[prop] let search: Signal<String> = Signal::new(String::new());
#[prop] let controls: Controls;
let (menu, live) = (controls.menu, controls.live);
let toggle_menu = move || menu.update(|open| *open = !*open);
</script>

<style>
/* The title is the spec's window x minus the sidebar's 240. The controls keep their distance from the window's right edge, the spec's 1280 minus their right edge, so they follow it as the window changes width. The live switch is the filter card's compare switch, x 976 and y 21 in a 1280 px window. */
.topbar { position: relative; flex: none; height: 64px; box-sizing: border-box; background-color: #ffffff; border-bottom: 1px solid #e5e7eb; }
.title { position: absolute; left: 24px; top: 0; height: 63px; display: flex; align-items: center; font-size: 20px; font-weight: 600; color: #111827; }
.search { display: flex; position: absolute; right: 324px; top: 14px; width: 280px; height: 36px; }
.search-input { overflow: visible; box-sizing: border-box; width: 280px; height: 36px; margin: 0; padding: 0 12px; border: 1px solid #d1d5db; border-radius: 8px; background-color: #f9fafb; color: #111827; font-size: 14px; }
.placeholder { position: absolute; left: 13px; top: 0; height: 36px; display: flex; align-items: center; color: #6b7280; font-size: 14px; pointer-events: none; }
.export-anchor { position: absolute; right: 84px; top: 14px; width: 96px; height: 36px; }
.export { width: 96px; height: 36px; box-sizing: border-box; margin: 0; padding: 0; border: none; border-radius: 8px; background-color: #4f46e5; color: #ffffff; font-size: 14px; font-weight: 600; display: flex; align-items: center; justify-content: center; }
.live-switch { display: block; position: absolute; right: 264px; top: 21px; width: 40px; height: 22px; margin: 0; accent-color: #4f46e5; --vapors-track-color: #d1d5db; }
.spinner { position: absolute; right: 216px; top: 20px; width: 24px; height: 24px; }
/* The turned square reaches 5 px past its box (24 * sqrt 2 = 34 wide): the svg is that much larger, centred on the box. */
.spinner svg { display: block; margin: -5px; }
.avatar { position: absolute; right: 24px; top: 14px; width: 36px; height: 36px; border-radius: 18px; background-color: #f59e0b; color: #ffffff; font-size: 13px; font-weight: 600; display: flex; align-items: center; justify-content: center; }
</style>

<div class="topbar">
  <div class="title">{title}</div>
  <div class="search">
    <input class="search-input" type="text" aria-label={placeholder} bind:value={search} />
    {if search.get().is_empty()}<div class="placeholder">{placeholder}</div>{/if}
  </div>
  <div class="export-anchor">
    <button class="export" onclick={toggle_menu}>{export}</button>
    <ExportMenu controls={controls} />
  </div>
  <input class="live-switch" type="checkbox" switch="" bind:checked={live.on} aria-label={live_label} />
  {if live.on.get()}<div class="spinner"><svg viewBox="-5 -5 34 34" width="34" height="34" aria-hidden="true"><path d={live.spinner.get()} fill="#4f46e5" /></svg></div>{/if}
  <div class="avatar">{initials}</div>
</div>
