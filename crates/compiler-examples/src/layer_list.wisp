<script>
#[prop]
let row: Snippet<(String, u32)> = Snippet::empty();
#[prop]
let names: Vec<(String, u32)> = Vec::new();
</script>

<ul>{for item in names.clone() key=item.0.clone()}<li>{render row(item.0.clone(), item.1)}</li>{/for}</ul>
