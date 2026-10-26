<script>
#[prop]
let calls: Signal<u32> = Signal::new(0);
let el: Signal<(String, String)> = Signal::new((String::from("foo"), String::from("bar")));
let invoked = move |el: (String, String)| {
    vapors_signals::untrack(|| calls.update(|n| *n += 1));
    el
};
</script>

{snippet foo(a: (String, String))}{a.get().0} {a.get().1}{/snippet}
{render foo(invoked(el))}
