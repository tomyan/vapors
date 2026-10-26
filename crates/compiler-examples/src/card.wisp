<script>
#[prop]
let title: String = String::new();
#[prop]
let children: Snippet = Snippet::empty();
</script>

<section><h2>{title}</h2>{render children()}<footer>end</footer></section>
