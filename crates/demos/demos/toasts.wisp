<script>
let next: Signal<u32> = Signal::new(1);
let toasts: Signal<Vec<(u32, String)>> = Signal::new(Vec::new());
let raise = move |message: &'static str| {
    let id = next.get();
    next.set(id + 1);
    toasts.update(|list| list.push((id, format!("{message} {id}"))));
};
let saved = move || raise("Saved version");
let failed = move || raise("Export failed, try again:");
</script>

<style>
/* Toasts above the shell (F2c S8): each button raises a toast into a
   fixed stack at the bottom right, over the canvas region, with a
   z-index above the shell's positioned bar. A toast stays until its
   Dismiss button is pressed. */
:global(html), :global(body) { height: 100%; }
:global(body) {
  margin: 0;
  display: grid;
  grid-template-rows: 44px minmax(0, 1fr);
  background-color: #e9ecef;
  font-size: 14px;
}
.bar {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 8px;
  background-color: #171b1f;
  color: #e4e8ec;
  box-shadow: 0 2px 6px #00000059;
}
.title { flex-grow: 1; }
button { cursor: pointer; padding: 6px 12px; border: none; border-radius: 6px; background-color: #3a7bd5; color: #ffffff; }
button:hover { background-color: #4a8be5; }
.canvas { background-color: #2b2b2b; }
.stack {
  position: fixed;
  right: 16px;
  bottom: 16px;
  z-index: 10;
  width: 260px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.toast {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 44px;
  padding: 0 8px 0 12px;
  background-color: #f1f3f5;
  color: #171b1f;
  border-left: 4px solid #2e7d32;
}
.message { flex-grow: 1; min-width: 0; }
.dismiss { background-color: #c7ccd1; color: #171b1f; }
.dismiss:hover { background-color: #b7bcc1; }
</style>

<div class="bar"><div class="title">touche</div><button onclick={saved}>Save</button><button onclick={failed}>Export</button></div>
<div class="canvas"></div>
<div class="stack">
  {for toast in toasts.get() key=toast.0}
    <div class="toast"><div class="message">{toast.get().1}</div><button class="dismiss" onclick={move |_event| { toasts.update(|list| list.retain(|t| t.0 != toast.get().0)); }}>Dismiss</button></div>
  {/for}
</div>
