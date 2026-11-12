<script>


#[prop]
let snippets: std::collections::HashMap<String, Snippet> = std::collections::HashMap::new();
#[prop]
let snippet: Signal<String> = Signal::new(String::new());
#[prop]
let optional: Memo<Option<Snippet>> = Memo::new(|| None);
let get_optional = move || optional.get();
</script>

{render snippets[&snippet]()}<hr>{render snippets.get(&snippet)()}<hr>{render snippets["foo"]()}<hr>{render snippets.get("foo")()}<hr>{render snippets.get("foo")()}<hr>{render snippets.get("foo")()}<hr>{render optional.or_else(|| snippets.get("bar").cloned())()}<hr>{render optional()}<hr>{render get_optional()()}
