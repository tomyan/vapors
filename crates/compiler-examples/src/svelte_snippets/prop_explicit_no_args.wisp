<script>
use super::no_args::{NoArgs, NoArgsProps};
</script>

{snippet foo()}<p>foo</p>{/snippet}
<NoArgs foo={foo.clone()} />
