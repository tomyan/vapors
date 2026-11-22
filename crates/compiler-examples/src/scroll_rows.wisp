<script>
// 20 rows of 20 px in a 60 px list: the rows at and below the first
// visible one exist, as many as fill the list and one more, after a
// spacer as tall as the rows above them.
let first: Signal<usize> = Signal::new(0);
let on_scroll = move |event: &mut Event| {
    if let Some([_, y]) = event.scroll_offset() {
        first.set(crate::scroll_rows_first(y));
    }
};
</script>

<style>
.list { width: 100px; height: 60px; overflow: auto; }
.content { height: 400px; }
.row { height: 20px; }
</style>

<div class="list" onscroll={on_scroll}>
  <div class="content">
    <div class="spacer" style:height={crate::scroll_rows_spacer(first.get())}></div>
    {for row in crate::scroll_rows_shown(first.get()) key=row}<div class="row">{row.get().to_string()}</div>{/for}
  </div>
</div>
