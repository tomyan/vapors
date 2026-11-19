<script>
let count: Signal<i32> = Signal::new(0);
</script>

<button onclick={move || count.update(|n| *n += 1)}>{count}</button>
