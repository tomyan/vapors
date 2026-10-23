<script>
#[prop]
let label: String = String::new();
</script>

<style>
/* One row of the panel. Its `.title` is its own: the panel's
   `.panel .title` does not reach it. */
.row {
  display: flex;
  align-items: center;
  height: 32px;
  padding: 0 8px;
  background-color: #f1f3f5;
  border-radius: 4px;
}
.title {
  width: 12px;
  height: 12px;
  margin-right: 8px;
  background-color: #2e7d32;
  border-radius: 6px;
}
.label { font-size: 14px; color: #171b1f; }
</style>

<div class="row">
  <div class="title"></div>
  <div class="label">{label}</div>
</div>
