<script>
#[prop]
let pick: Signal<bool> = Signal::new(true);
#[prop]
let word: Signal<String> = Signal::new(String::new());
</script>

{snippet bold(said: String)}<b>{said}</b>{/snippet}
{snippet italic(said: String)}<i>{said}</i>{/snippet}
<p>{render (if pick.get() { bold.clone() } else { italic.clone() })(word)}</p>
