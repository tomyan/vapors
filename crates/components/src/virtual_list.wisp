<script>
// A vertical list that holds only the rows in and near its window: every
// row `row_height` px tall, and `overscan` rows kept beyond each edge of
// the window (virtual_window.rs). The window is the container's laid-out
// height, which each `resize` reports, so the rows follow the list's
// size as its page resizes. `height` above 0 sets that height in px;
// at 0, the default, the list fills its parent's height (`height:
// 100%`), so the parent gives it one. `keys` are the rows' stable
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
// Until the first `resize`, the height given; a filling list's first
// update holds only the overscan, and the frame's second the window.
let laid_out: Signal<f32> = Signal::new(height);
let range = move || keys.with(|all| window.with_height(laid_out.get()).rows(offset.get(), all.len()));
let shown: Memo<Vec<usize>> = Memo::new(move || keys.with(|all| all[range()].to_vec()));
let above: Memo<f32> = Memo::new(move || window.px(range().start));
let total: Memo<f32> = Memo::new(move || window.px(keys.with(Vec::len)));
let fixed = height > 0.0;
let on_scroll = move |event: &mut Event| {
    if let Some([_, y]) = event.scroll_offset() {
        offset.set(y);
    }
};
// A resize carries the offset as laid out, which a taller list at its
// end clamped before its `scroll` (the next frame's) can say.
let on_resize = move |event: &mut Event| {
    if let Some([_, h]) = event.client_size()
        && h.to_bits() != laid_out.get_untracked().to_bits()
    {
        laid_out.set(h);
    }
    if let Some([_, y]) = event.scroll_offset()
        && y.to_bits() != offset.get_untracked().to_bits()
    {
        offset.set(y);
    }
};
</script>

<style>
.virtual-list { overflow-y: auto; }
.virtual-list.fill { height: 100%; }
</style>

{if fixed}
<div class="virtual-list" style:height={height} onscroll={on_scroll} onresize={on_resize}>
  <div class="virtual-content" style:height={total.get()}>
    <div class="virtual-above" style:height={above.get()}></div>
    {for key in shown.get() key=key}<div class="virtual-row" style:height={row_height}>{render row(key.get())}</div>{/for}
  </div>
</div>
{else}
<div class="virtual-list fill" onscroll={on_scroll} onresize={on_resize}>
  <div class="virtual-content" style:height={total.get()}>
    <div class="virtual-above" style:height={above.get()}></div>
    {for key in shown.get() key=key}<div class="virtual-row" style:height={row_height}>{render row(key.get())}</div>{/for}
  </div>
</div>
{/if}
