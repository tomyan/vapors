<script>
#[prop]
let label: String = String::new();
#[prop]
let value: String = String::new();
#[prop]
let marked: bool = false;
</script>

<style>
.row { display: flex; height: 24px; gap: 8px; }
.row.marked { background-color: #eef; }
.label { color: #222; }
.value { width: 80px; height: 20px; overflow: hidden; }
button { width: 60px; height: 20px; background-color: #ddd; }
button:hover { background-color: #bbb; }
button:focus { outline: 2px solid #36c; }
</style>

<div class="row" class:marked={marked}><span class="label">{label}</span><span class="value">{value}</span><button>Edit</button></div>
