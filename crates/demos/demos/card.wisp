<script>
#[prop]
let title: String = String::new();
#[prop]
let children: Snippet = Snippet::empty();
</script>

<style>
/* A card (1-R1): a title row over whatever its parent passes it as
   children, which it places with {render children()}. */
section { margin: 8px; background-color: #ffffff; border: 1px solid #c7ccd1; border-radius: 6px; }
h2 { margin: 0; padding: 8px 12px; font-size: 14px; background-color: #171b1f; color: #e4e8ec; border-radius: 5px 5px 0 0; }
.body { display: flex; align-items: center; gap: 12px; padding: 8px 12px; }
</style>

<section><h2>{title}</h2><div class="body">{render children()}</div></section>
