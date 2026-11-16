<script>
use crate::bound_word::{BoundWord, BoundWordProps};
#[prop]
let word: Signal<String> = Signal::new(String::new());
</script>

<p><BoundWord word={word} /></p>
