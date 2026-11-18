# The .wisp format

Every example on this page was compiled with `vapors-compiler` at the
pinned rev and mounted into a document whose first update reported no
refusal. The full rules, with every refusal, are in
[docs/components.md](https://github.com/tomyan/vapors/blob/main/docs/components.md);
what CSS, elements and events exist is in [supported.md](https://github.com/tomyan/vapors/blob/main/plugins/vapors/skills/vapors/reference/supported.md).

A component is one file: an optional `<script>` (Rust), an optional
`<style>` (CSS), then the template (HTML). The file name gives the type:
`text-field.wisp` or `text_field.wisp` is `TextField`, in the module
`text_field`, with props struct `TextFieldProps`.

`counter.wisp`:

```html
<script>
let count: Signal<i32> = Signal::new(0);
let increment = move || count.update(|n| *n += 1);
</script>

<style>
/* a button is laid out only as a flex or grid item */
.row { display: flex; }
button { padding: 4px 8px; }
</style>

<div class="row"><button onclick={increment}>Clicked {count} times</button></div>
```

## Script

The script is Rust statements that run once, in order, when the
component mounts. Some `let` forms mean more:

| Statement | Becomes |
|---|---|
| `let n: Signal<i32> = Signal::new(0);` | a public field `n` of the component struct; the type must be written |
| `let total: Memo<f64> = Memo::new(move \|\| ..);` | a public field, recomputed when what it reads changes |
| `#[prop] let name: String = "world".into();` | a field of the props struct, with that default |
| `let go = move \|\| ..;` | a handler; a closure with no parameters may still be used as a listener |
| `use crate::card::{Card, CardProps};` | brings a child component (or any item) into scope |
| `#[dispatcher(pick)] let events;` | sends the component's own events (below) |

In scope without a `use`: `Signal`, `Memo`, `Effect`, `Owner`, `Event`,
`NodeId`, `Snippet`. Other items need a path or a `use`
(`vapors_signals::batch`, `vapors_signals::untrack`).

A type that a signal, memo or prop holds must be declared in a Rust
module of the crate and imported: a `struct` declared inside the
script is local to the mount function, and the component struct cannot
name it (rustc: "cannot find type").

`derived.wisp`:

```html
<script>
let price: Signal<f64> = Signal::new(4.0);
let quantity: Signal<u32> = Signal::new(3);
let total: Memo<f64> = Memo::new(move || price.get() * f64::from(quantity.get()));
// runs once now, then whenever `total` changes
Effect::new(move || println!("total is now {}", total.get()));
let more = move || quantity.update(|q| *q += 1);
</script>

<style>
.column { display: flex; flex-direction: column; gap: 8px; }
</style>

<div class="column">
  <p>{quantity} at {price} costs {total}</p>
  <button onclick={more}>One more</button>
</div>
```

- `signal.get()` reads and subscribes; `get_untracked()` reads without
  subscribing; `set(v)`, `update(|v| ..)`, `set_if_changed(v)` write;
  `with(|v| ..)` borrows.
- `Effect::new(move || ..)` runs now and again whenever a signal it read
  changes. A signal it writes is picked up after it returns.
- `Signal::derived(move || ..)` is a read-only `ReadSignal<T>` computed
  from others (`get`, `with`, no `set`), for a prop that takes a
  `ReadSignal<T>` (a 3D node's `transform`). `signal.read_only()` makes
  a view of a signal; a `Signal` or `Memo` binding passed alone as a
  prop converts by itself, any other expression calls `.read_only()`.
  A prop typed `Signal<T>` lets the child write: it is a grant, as
  Svelte's `$bindable` is.
- Signals, memos and effects belong to the component and stop when it
  unmounts. Handles are `Copy`, so closures capture them by `move`.

## Template

### Text and holes

`{expr}` is any Rust expression. In a text hole a bare signal or memo
name is read for you: `{count}` means `count.get()`, and
`{items.len()}` means `items.get().len()`. Everywhere else (attribute
values, `{if}` conditions, `{for}` lists) write `.get()` yourself. Each
hole updates its own text node when what it reads changes.

### Attributes

`name="text"` is fixed; `name={expr}` is set again whenever `expr`
changes; `{name}` is short for `name={name}`. A boolean attribute
(`disabled`, `checked`, ...) cannot come from an expression: use
`bind:checked`, or put the element inside `{if}`.

### Conditions

`{if cond} .. {else} .. {/if}`. There is no `{else if}`: nest an `{if}`
inside the `{else}`.

`conditions.wisp`:

```html
<script>
let stock: Signal<u32> = Signal::new(3);
let sell = move || stock.update(|n| *n = n.saturating_sub(1));
</script>

<style>
.column { display: flex; flex-direction: column; gap: 8px; }
</style>

<div class="column">
  {if stock.get() == 0}
    <p>Sold out</p>
  {else}
    {if stock.get() == 1}
      <p>Last one</p>
    {else}
      <p>{stock} left</p>
    {/if}
  {/if}
  <button onclick={sell}>Sell one</button>
</div>
```

### Lists

`{for x in expr} .. {/for}` or `{for (i, x) in expr key=expr} .. {/for}`.
Without `key=`, rows are matched by index. The `key=` expression sees
each item as a plain value. Inside the row, `x` and `i` are memos:
bare in a text hole, `x.get()` elsewhere. Two rows with the same key
are refused when the list changes. With `(i, x)` and a `key=`, rustc
warns that `i` is unused in the generated key closure; the warning is
harmless.

`lists.wisp`:

```html
<script>
use crate::Task;

let tasks: Signal<Vec<Task>> = Signal::new(vec![
    Task { id: 1, title: "Write the page".into(), done: true },
    Task { id: 2, title: "Publish it".into(), done: false },
]);
let reverse = move || tasks.update(|all| all.reverse());
</script>

<style>
.column { display: flex; flex-direction: column; gap: 8px; }
</style>

<div class="column">
  {for (i, task) in tasks.get() key=task.id}
    <p class="row" class:done={task.get().done}>{i}. {task.get().title}</p>
  {/for}
  {for word in ["a", "b", "c"]}<span>{word}</span>{/for}
  <button onclick={reverse}>Reverse</button>
</div>
```

### Classes and inline style

`class="a b"` is fixed. `class:name={bool}` adds `name` while the
expression is true. `style="..."` may be written as literal text.
`class={..}` and `style={..}` are refused: the compiler must see every
class and declaration an element can have. To move or resize something
every frame, use the 3D scene ([runtime.md](https://github.com/tomyan/vapors/blob/main/plugins/vapors/skills/vapors/reference/runtime.md)).

`classes.wisp`:

```html
<script>
let on: Signal<bool> = Signal::new(false);
let toggle = move || on.update(|v| *v = !*v);
</script>

<style>
.lamp { width: 40px; height: 40px; background-color: #333; }
.lamp.lit { background-color: #fc3; }
</style>

<div class="lamp" class:lit={on.get()} style="border-radius: 20px" onclick={toggle}></div>
```

### Events

`on<event>={handler}`. The handler is a script closure or an inline
`move |event: &mut Event| ..`. It returns `()` or `Result<(), E>` with
`E: Display`; an `Err` is shown in the page's `#error`. Events bubble,
as in the DOM. The events the tree fires are listed in
[supported.md](https://github.com/tomyan/vapors/blob/main/plugins/vapors/skills/vapors/reference/supported.md#events); a listener for any other name is
refused, except a component's own events.

`Event` has `key()` and `code()` (DOM names: `"ArrowUp"`, `" "`,
`"KeyA"`), `repeat()`, `modifiers()`, `offset()` and `client()`
(`[x, y]` in CSS pixels), `value()`, `value_as_number()`, `checked()`,
`target()`, `prevent_default()`, `stop_propagation()` and
`set_pointer_capture()`.

Keys go to the focused element (one with `tabindex`, a control) and
bubble up from there; with nothing focused they go to `<body>`.

`events.wisp`:

```html
<script>
let last: Signal<String> = Signal::new(String::new());
let at: Signal<String> = Signal::new(String::new());
let key = move |event: &mut Event| {
    last.set(event.key().to_string());
    if event.key() == " " {
        event.prevent_default();
    }
};
let moved = move |event: &mut Event| {
    let [x, y] = event.offset();
    at.set(format!("{x:.0}, {y:.0}"));
};
let save = move |_: &mut Event| -> Result<(), String> {
    if last.get().is_empty() { return Err("press a key first".into()); }
    Ok(())
};
</script>

<style>
.column { display: flex; flex-direction: column; gap: 8px; }
</style>

<div class="column" tabindex="0" onkeydown={key} onpointermove={moved}>
  <p>Last key: {last}; pointer at {at}</p>
  <button onclick={save}>Save</button>
</div>
```

### Keys for the whole page: `<vapors:window>`

To read keys whatever has focus, put `<vapors:window onkeydown={..}
onkeyup={..} />` at the top level of a template. It hears every key
event the page gets, after the focused element's listeners (an event they
stopped with `stop_propagation` does not reach it). The handlers take
`&mut Event` or nothing. One per component; not inside an element,
`{if}`, `{for}`, a snippet or a component tag. Call
`event.prevent_default()` for the keys the page uses.

`window_keys.wisp`:

```html
<script>
let x: Signal<i32> = Signal::new(0);
let step = move |event: &mut Event| {
    match event.key() {
        "ArrowLeft" => x.update(|v| *v -= 1),
        "ArrowRight" => x.update(|v| *v += 1),
        _ => return,
    }
    event.prevent_default();
};
</script>

<p>Position {x}</p>
<vapors:window onkeydown={step} />
```

### Bindings

`bind:value={signal}` on `input` (a `Signal<String>`, or `Signal<f64>`
when `type` is `range` or `number`), `bind:checked={signal}` on a
checkbox or radio, and `bind:this={signal}` on any element for its
`NodeId`. The `type` must be literal.

`bindings.wisp`:

```html
<script>
let name: Signal<String> = Signal::new(String::new());
let volume: Signal<f64> = Signal::new(50.0);
let muted: Signal<bool> = Signal::new(false);
let field: Signal<Option<NodeId>> = Signal::new(None);
</script>

<style>
.column { display: flex; flex-direction: column; gap: 8px; }
/* a text field has no default width */
input[type="text"] { width: 12em; }
</style>

<div class="column">
  <input type="text" bind:value={name} bind:this={field} />
  <input type="range" min="0" max="100" bind:value={volume} />
  <input type="checkbox" bind:checked={muted} />
  <p>{name} at {volume}{if muted.get()} (muted){/if}</p>
</div>
```

### Child components and props

A capitalised tag is a component. Bring it into scope with a `use` of
the type and its props struct. Attributes become props; a prop left out
takes its default. A prop is read once at mount unless its type is
`Signal<T>`.

`greeting.wisp`:

```html
<script>
#[prop] let name: String = "world".into();
#[prop] let excited: bool = false;
// a Signal prop stays live: the parent's later writes show here
#[prop] let visits: Signal<u32> = Signal::new(0);
</script>

<p>Hello, {name}{if excited}!{/if} Visit {visits}.</p>
```

`props_parent.wisp`:

```html
<script>
use crate::greeting::{Greeting, GreetingProps};
let visits: Signal<u32> = Signal::new(1);
</script>

<div>
  <Greeting name="Ada" excited={true} visits={visits} />
  <Greeting />
</div>
```

### Children and snippets

Markup inside a component tag is its `children` prop, which the child
declares as a `Snippet` and places with `{render children()}`.
`{snippet name(param: Type, ..)} .. {/snippet}` declares a reusable
piece of template; `{render name(args)}` builds it. Parameter types are
owned (`String`, not `&str`) and `Clone + PartialEq`.

`card.wisp`:

```html
<script>
#[prop] let title: String = String::new();
#[prop] let children: Snippet = Snippet::empty();
</script>

<style>
.card { padding: 12px; background-color: #222; }
</style>

<div class="card">
  <h2>{title}</h2>
  {render children()}
</div>
```

`snippets.wisp`:

```html
<script>
use crate::card::{Card, CardProps};
let names: Signal<Vec<String>> = Signal::new(vec!["Ada".into(), "Bo".into()]);
</script>

<div>
  {snippet badge(name: String, rank: usize)}
    <span class="badge">{rank}. {name}</span>
  {/snippet}
  <Card title={"People".to_string()}>
    {for (i, name) in names.get()}{render badge(name.get(), i.get() + 1)}{/for}
  </Card>
</div>
```

### A component's own events

`#[dispatcher(name, ..)] let events;` declares events the component
fires with `events.dispatch(node, Event::new("name"))`; they bubble
like any other, so a parent listens with `on<name>` on an element
around the component.

`picker.wisp`:

```html
<script>
#[dispatcher(pick)]
let events;
let button: Signal<Option<NodeId>> = Signal::new(None);
let click = move |_: &mut Event| -> Result<(), String> {
    let Some(node) = button.get_untracked() else { return Ok(()) };
    events.dispatch(node, Event::new("pick")).map(|_| ()).map_err(|refused| refused.to_string())
};
</script>

<button bind:this={button} onclick={click}>Pick</button>
```

`picker_parent.wisp`:

```html
<script>
use crate::picker::{Picker, PickerProps};
let picks: Signal<u32> = Signal::new(0);
let picked = move |_: &mut Event| picks.update(|n| *n += 1);
</script>

<style>
.column { display: flex; flex-direction: column; }
</style>

<div class="column" onpick={picked}>
  <Picker />
  <p>Picked {picks} times</p>
</div>
```

## The frame clock

`vapors_elements::frame_time()` is a `ReadSignal<Duration>`: the time since
the first frame, set once at the start of each frame.
`frame_delta()` is the time since the previous frame (zero on the
first; after an idle wait it includes the wait). While anything reads
either, the page draws every display refresh (on the web, each
`requestAnimationFrame`); when the last reader unmounts it goes idle
again.

`clock.wisp`:

```html
<script>
use vapors_elements::{frame_delta, frame_time};
let time = frame_time();
let delta = frame_delta();
// a value that follows the clock: recomputed every frame
let angle: Memo<f32> = Memo::new(move || time.get().as_secs_f32() * 1.5);
let fps: Memo<u32> = Memo::new(move || {
    let dt = delta.get().as_secs_f32();
    if dt > 0.0 { (1.0 / dt).round() as u32 } else { 0 }
});
</script>

<p>Turned {angle} radians at {fps} frames a second</p>
```

## Style

A component's `<style>` applies only to the elements it creates.
`:global(selector)` reaches outside it, as `:global(body)` does.
`@keyframes` names are global.

`scoped.wisp`:

```html
<style>
/* only this component's <p> elements */
p { color: #ccc; }
/* outside the component: the page's body */
:global(body) { margin: 0; background-color: #111; }
</style>

<p>Styled here only.</p>
```

## Layout rules that refuse at run time

These compile, then the page shows an error in `#error`:

- A `<button>` or `<input>` is laid out only as a flex or grid item, so
  its parent needs `display: flex` (or `grid`). The examples above wrap
  them in `.column { display: flex; flex-direction: column; }`.
- A text or number `<input>` needs a width (`width: 12em`).
- `br`, lists (`ul`, `li`), tables, `label`, `select` and `textarea` are
  not drawn yet; see [supported.md](https://github.com/tomyan/vapors/blob/main/plugins/vapors/skills/vapors/reference/supported.md#elements).

## Unsupported

HTML comments (use `/* */` in `<style>` or `//` in `<script>`),
`<title>`, slots (use `children` and snippets), `{else if}`, a Rust
`if` expression inside a hole (`{if a { x } else { y }}`: compute it in
a memo), and recursive snippets.
