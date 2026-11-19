<script>
#[prop] let label: String = String::new();
#[prop] let index: usize = 0;
#[prop] let badge: u32 = 0;
#[prop] let active: Signal<usize> = Signal::new(0);
let select = move || active.set(index);
</script>

<style>
.item { display: flex; align-items: center; gap: 12px; height: 40px; margin-bottom: 4px; padding: 0 12px 0 16px; border-radius: 8px; color: #d1d5db; font-size: 14px; }
.item:hover { background-color: #374151; }
.item.current, .item.current:hover { background-color: #1f2937; }
.icon { width: 16px; height: 16px; flex: none; border-radius: 4px; background-color: #4b5563; }
.current .icon { background-color: #818cf8; }
.nav-label { flex: 1; }
.badge { height: 20px; padding: 0 7px; border-radius: 10px; background-color: #ef4444; color: #ffffff; font-size: 12px; font-weight: 600; display: flex; align-items: center; }
</style>

<div class="item" class:current={active.get() == index} onclick={select}>
  <div class="icon"></div>
  <div class="nav-label">{label}</div>
  {if badge > 0}<div class="badge">{badge}</div>{/if}
</div>
