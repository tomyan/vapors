<script>
#[prop] let label: String = String::new();
#[prop] let index: usize = 0;
#[prop] let series: Signal<usize> = Signal::new(0);
let select = move || series.set(index);
</script>

<style>
.segment { width: 64px; height: 28px; border-radius: 6px; display: flex; align-items: center; justify-content: center; font-size: 13px; font-weight: 600; color: #6b7280; }
.segment.selected { background-color: #ffffff; color: #111827; box-shadow: 0 1px 3px #11182740; }
</style>

<div class="segment" class:selected={series.get() == index} onclick={select}>{label}</div>
