<script>
#[prop]
let prop: String = String::new();
#[prop]
let children: Snippet = Snippet::empty();
</script>

<div>{prop}{render children()}</div>
