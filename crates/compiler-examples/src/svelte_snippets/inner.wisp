<script>
#[prop]
let snippet: Memo<Snippet> = Memo::new(Snippet::empty);
</script>

{render snippet()}
