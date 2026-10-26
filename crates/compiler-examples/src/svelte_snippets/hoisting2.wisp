<script>
let abc: String = String::from("a");
</script>

{render b()}
{snippet a()}{abc}{/snippet}
{snippet b()}{render a()}{/snippet}
