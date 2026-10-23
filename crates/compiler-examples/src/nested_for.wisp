<script>
let rows: Signal<Vec<u32>> = Signal::new(vec![1, 2, 3]);
</script>

<ul>{for r in rows.get() key=r}{if true}<li>{r}</li>{/if}{/for}</ul>
