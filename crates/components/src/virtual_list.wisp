<script>
// A vertical list that holds only the rows in and near its window: every
// row `row_height` px tall, the window `height` px, and `overscan` rows
// kept beyond each edge (virtual_window.rs). `keys` are the rows' stable
// identities in the order shown; each existing row is `row(key)` in a
// box of the row height, kept while its key stays near the window, moved
// with its key when `keys` reorder. The container scrolls as any
// `overflow-y: auto` box does; its `scroll` gives the offset.
// Every prop has a default because a tag from another crate cannot pass
// a required one yet (docs/components.md, "Required props"); a zero
// row_height is refused at mount.
use crate::virtual_window::Window;

#[prop]
let keys: ReadSignal<Vec<usize>> = Signal::new(Vec::new()).read_only();
#[prop]
let row_height: f32 = 0.0;
#[prop]
let height: f32 = 0.0;
#[prop]
let overscan: usize = 4;
#[prop]
let row: Snippet<(usize,)> = Snippet::empty();

let window = Window::new(row_height, height, overscan);
let offset: Signal<f32> = Signal::new(0.0);
let range = move || keys.with(|all| window.rows(offset.get(), all.len()));
let shown: Memo<Vec<usize>> = Memo::new(move || keys.with(|all| all[range()].to_vec()));
let above: Memo<f32> = Memo::new(move || window.px(range().start));
let total: Memo<f32> = Memo::new(move || window.px(keys.with(Vec::len)));
let on_scroll = move |event: &mut Event| {
    if let Some([_, y]) = event.scroll_offset() {
        offset.set(y);
    }
};
</script>

<style>
.virtual-list { overflow-y: auto; }
</style>

<div class="virtual-list" style:height={height} onscroll={on_scroll}>
  <div class="virtual-content" style:height={total.get()}>
    <div class="virtual-above" style:height={above.get()}></div>
    {for key in shown.get() key=key}<div class="virtual-row" style:height={row_height}>{render row(key.get())}</div>{/for}
  </div>
</div>
