<script>
let shown: Signal<bool> = Signal::new(true);
</script>

{if shown.get()}<p>shown</p>{/if}
