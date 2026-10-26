<script>
#[prop]
let numbers: Signal<Vec<i32>> = Signal::new(vec![1, 2, 3]);
</script>

<button onclick={move |_| { numbers.update(|list| list.push(list.last().map_or(1, |n| n + 1))); }}>push</button>
<div style="display: grid; grid-template-columns: 1fr 1fr">
	<div>
		{snippet x(n: i32)}<p style="color: red">{n}</p>{/snippet}
		{for n in numbers.get()}{render x(n)}{/for}
	</div>
	<div>
		{snippet x(n: i32)}<p style="color: blue">{n}</p>{/snippet}
		{for n in numbers.get()}{render x(n)}{/for}
	</div>
</div>
