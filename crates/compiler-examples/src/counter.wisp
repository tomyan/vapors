<script>
let count: Signal<i32> = Signal::new(0);
let increment = move || count.update(|n| *n += 1);
</script>

<style>
button { padding: 4px 8px; }
</style>

<button onclick={increment}>Clicked {count} times</button>
