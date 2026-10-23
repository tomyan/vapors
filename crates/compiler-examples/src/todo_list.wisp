<script>
use crate::todo_item::{TodoItem, TodoItemProps};
let items: Signal<Vec<(u32, String)>> = Signal::new(Vec::new());
let draft: Signal<String> = Signal::new(String::new());
</script>

<input bind:value={draft} />
{if items.get().is_empty()}
  <p>Nothing to do</p>
{else}
  <ul>
    {for item in items.get() key=item.0}
      <TodoItem label={item.get().1} />
    {/for}
  </ul>
{/if}
