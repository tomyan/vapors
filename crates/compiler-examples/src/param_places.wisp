<script>
#[prop]
let n: Signal<i32> = Signal::new(1);
</script>

{snippet show(count: i32)}<p title={count.get().to_string()}>{for i in 0..count.get() key=i}<i>{i}</i>{/for}{if count.get() > 1}<b>many</b>{/if}</p>{/snippet}
{render show(n)}{render show(3)}
