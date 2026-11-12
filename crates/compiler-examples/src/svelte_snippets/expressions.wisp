<script>
use std::collections::HashMap;

use super::expressions_child::{ExpressionsChild, ExpressionsChildProps};

let snippet: Signal<String> = Signal::new(String::from("foo"));
let show: Signal<bool> = Signal::new(false);
</script>

{snippet snip_foo()}<p>foo</p>{/snippet}
{snippet snip_bar()}<p>bar</p>{/snippet}
<ExpressionsChild snippets={HashMap::from([(String::from("foo"), snip_foo.clone()), (String::from("bar"), snip_bar.clone())])} snippet={snippet} optional={Memo::new({ let snip_foo = snip_foo.clone(); move || show.get().then(|| snip_foo.clone()) })} />
<button onclick={move |_| { snippet.set(String::from("bar")); show.set(true); }}>toggle</button>
