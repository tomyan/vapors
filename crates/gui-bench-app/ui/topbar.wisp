<script>
use crate::controls::Controls;
use crate::dashboard::{EXPORT_LABEL, PAGE_TITLE, SEARCH_PLACEHOLDER, USER_INITIALS};
let (title, placeholder, export, initials) = (PAGE_TITLE, SEARCH_PLACEHOLDER, EXPORT_LABEL, USER_INITIALS);
#[prop] let search: Signal<String> = Signal::new(String::new());
#[prop] let controls: Controls = Controls::unbound();
let controls = controls.bound("Topbar");
let menu: Signal<bool> = controls.menu;
let toggle_menu = move || menu.update(|open| *open = !*open);
</script>

<style>
/* Positions are the spec's window x minus the sidebar's 240. The live switch (x 976) is a later slice's. */
.topbar { position: relative; flex: none; height: 64px; box-sizing: border-box; background-color: #ffffff; border-bottom: 1px solid #e5e7eb; }
.title { position: absolute; left: 24px; top: 0; height: 63px; display: flex; align-items: center; font-size: 20px; font-weight: 600; color: #111827; }
.search { display: flex; position: absolute; left: 436px; top: 14px; width: 280px; height: 36px; }
.search-input { overflow: visible; box-sizing: border-box; width: 280px; height: 36px; margin: 0; padding: 0 12px; border: 1px solid #d1d5db; border-radius: 8px; background-color: #f9fafb; color: #111827; font-size: 14px; }
.export { position: absolute; left: 860px; top: 14px; width: 96px; height: 36px; box-sizing: border-box; margin: 0; padding: 0; border: none; border-radius: 8px; background-color: #4f46e5; color: #ffffff; font-size: 14px; font-weight: 600; display: flex; align-items: center; justify-content: center; }
.avatar { position: absolute; left: 980px; top: 14px; width: 36px; height: 36px; border-radius: 18px; background-color: #f59e0b; color: #ffffff; font-size: 13px; font-weight: 600; display: flex; align-items: center; justify-content: center; }
</style>

<div class="topbar">
  <div class="title">{title}</div>
  <div class="search">
    <input class="search-input" type="text" aria-label={placeholder} placeholder={placeholder} bind:value={search} />
  </div>
  <button class="export" onclick={toggle_menu}>{export}</button>
  <div class="avatar">{initials}</div>
</div>
