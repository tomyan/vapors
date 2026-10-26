<script>
use super::inner::{Inner, InnerProps};

let show_foo: Signal<bool> = Signal::new(true);
</script>

{snippet snip_foo()}<p>foo</p>{/snippet}
{snippet snip_bar()}<p>bar</p>{/snippet}
<Inner snippet={Memo::new({ let snip_foo = snip_foo.clone(); let snip_bar = snip_bar.clone(); move || if show_foo.get() { snip_foo.clone() } else { snip_bar.clone() } })} />
<button onclick={move |_| show_foo.set(false)}>show bar</button>
