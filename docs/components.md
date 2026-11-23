# Components: `.wisp` files

A component is one `.wisp` file with a `<script>` in Rust, a `<style>`
in CSS and an HTML template. `vapors-compiler` turns it into a Rust
file from `build.rs`. The generated code calls the `Dom` trait in
`vapors-dom` and uses the signals in `vapors-signals`. It is plain Rust,
formatted by prettyplease, and can be read and stepped through in a
debugger.

The model is sumi's (`~/projects/sumi/docs/`), with Rust in place of
Go. This page lists what the compiler does today and where it differs
from sumi.

```html
<script>
let count: Signal<i32> = Signal::new(0);
let increment = move || count.update(|n| *n += 1);
</script>

<style>
button { padding: 4px 8px; }
</style>

<button onclick={increment}>Clicked {count} times</button>
```

That is `crates/compiler-examples/src/counter.wisp`. Its generated code
is pinned in
`crates/compiler/tests/snapshots/codegen_snapshot__counter_component_generates_readable_rust.snap`.

## The file

- `<script>` and `<style>` are optional, at most one of each, and come
  before the template in either order. Neither section tag takes
  attributes.
- The template is everything after them and must not be empty.
- The file name gives the type name: `counter.wisp` is `Counter`, and
  `text-input.wisp` or `text_input.wisp` is `TextInput`.

## Script

The script is a sequence of Rust statements. They run once, in order,
when the component mounts. Some `let` forms have a special meaning:

| Statement | Becomes |
|---|---|
| `let count: Signal<i32> = Signal::new(0);` | a public field `count: Signal<i32>` on the component struct. The signal is created with `Signal::named("count", ..)`, so refusals name it. |
| `let doubled: Memo<i32> = Memo::new(..);` | a public field, like a signal |
| `#[prop] let label: String = "Count".into();` | a field of `CounterProps`, with that value as its default |
| `#[prop] let controls: Controls;` | a required prop: no default, a field `controls: Required<Controls>` (`Required::Missing` in `Default`). See "Required props" |
| `let increment = move \|\| ..;` | a handler. A closure with no parameters is called without the event, in the script or inline in the template. |
| `use crate::todo_item::{TodoItem, TodoItemProps};` | brings a child component into scope for its tag |
| `#[dispatcher(pick)] let events;` | a `vapors_dom::Dispatcher` for the component's own events (doc 03, "A component's own events"); at most one, with no other attribute on that `let` |

A signal or memo needs its type written out, because it becomes a struct
field. `let count = Signal::new(0)` is refused, and the message gives
the annotated form. The brief's examples omit the annotation. That is
the one difference from the brief in this slice.

## Template

- **Elements** come from a fixed HTML vocabulary, listed in
  `crates/compiler/src/template/vocabulary.rs`. Void elements such as
  `input` and `br` close themselves. Any other lowercase tag is refused.
- **Text** may hold `{expr}` holes. One rule covers whitespace. A
  whitespace-only run between two non-text things (an element, an
  opening or closing tag, or a hole) is dropped when it contains a
  newline, and becomes one space otherwise. A run with anything else in
  it is kept as written. So `<p>\n  hello {name}\n</p>` gives the
  static text `"\n  hello "` and the hole's own text node, with nothing
  after it; `{a} {b}` keeps its space; `{a}` and `{b}` on separate lines
  have none between them; and in `<b>a</b>` followed by `text` on the
  next line, the text keeps its newline and indent. `&nbsp;` is never
  collapsed. A hole is always its own text node, holding exactly its
  value.
- **Character references** in text and quoted attribute values are
  decoded: `&amp;` `&lt;` `&gt;` `&quot;` `&apos;` `&nbsp;`, `&#NN;`
  and `&#xNN;`. An unknown `&name;`, or a number that names no
  character, is refused at the `&`. An `&` that does not start
  `&name;` is kept as written, so `salt & pepper` needs no escaping.
- **Holes** hold any Rust expression, including string and raw string
  literals that contain `{`, `}` or quotes. A whole Rust
  if-expression, `{if a { x } else { y }}`, is refused, because `{if`
  opens a template block. Compute the value in a memo or a script
  closure, or use `{if}..{else}..{/if}`.
- **Attributes** come in three forms: `name="literal"`,
  `name={expr}`, and the shorthand `{name}`, which means
  `name={name}`. A bare `name` with no value is refused with the fix.
  Each name may appear once. A quoted value is text: a `{` in it is
  refused (build the value with `name={format!(..)}`, or write
  `&#123;`). A literal attribute is part of the element's style key:
  the tree sets it at creation and refuses `set_attribute` of it
  afterwards, naming `name={expr}`, as it refuses `class` and `style`,
  so the build-time analysis can read it (the analysable-sources
  decision, `docs/decisions/2026-11-11-analysable-sources.md`); an
  expression attribute is a run-time input. Attribute names are
  matched as written (`Title="a"` is not `[title]`); the refusal
  compares them without case.
- **Events:** `onclick={handler}` listens for `click`. The event name
  is the attribute name after `on`. The handler is a script closure or
  an inline closure that takes `&mut Event` or no parameters
  (`onclick={move || hop()}` is called without the event, as a script
  closure with no parameters is); an inline closure with two or more
  parameters is refused at compile time, naming the file and line. On a
  component tag `onclick={..}` is still an ordinary prop, passed as
  written (tag listeners, doc 2026-11-18, are not built yet). It returns `()` or
  `Result<(), E>` with `E: Display`, so it may use `?`; an `Err` goes to
  the app's error report. A handler is `Fn`, since a listener may run
  again inside itself when it dispatches: it keeps its state in a
  signal or a `Cell`, and one that assigns to a binding it captures
  (`n += 1`, `&mut n`) is refused, naming the binding. Events bubble.
  `event.stop_propagation()` and `event.prevent_default()` work as in
  the DOM.
- **Scroll:** `onscroll={handler}` on a scroll container (`overflow:
  auto` or `scroll`) hears `scroll` once per frame, before the frame's
  update, when its offset has changed since its last `scroll`: by wheel,
  touch, fling, `Document::scroll_to`, a press's scroll into view, or an
  update clamping the offset to content that shrank. A scroll that leaves
  the offset where it was (a `scroll_to` the same offset, a wheel at the
  end of the range, a scroll and its reverse in one frame) fires none.
  `event.scroll_offset()` is `Some([x, y])`, the offset in CSS pixels;
  a change the handler makes is drawn in the same frame. It does not
  bubble, as an element's `scroll` does not in the DOM. A host that runs
  its own frames calls `Document::settle_scroll` before each update;
  until it does, `Document::take_changed` answers true.
- **Resize:** `onresize={handler}` on a scroll container hears `resize`
  in the same settle, after the `scroll`s, when the last update laid it
  out at a client size (its padding box) other than its last `resize`
  carried, including its first layout at any size above (0, 0).
  `event.client_size()` is `Some([width, height])` in CSS pixels. Only
  a container with a `resize` listener is observed: one without owes
  none and never asks for a frame. It
  does not bubble, and other elements never fire it: the DOM has no
  element `resize`, and this stands in for a `ResizeObserver` on a
  scroll container. The frame that lays a new size out draws before the
  `resize`, so what the handler changes is drawn in the frame after.
- **Classes:** `class:name={bool}` gives the element class `name`
  while the expression is true, beside the names in a literal
  `class="..."`; one effect sets the `class` attribute from both.
- **Style bindings:** `style:height={expr}` sets one CSS property of
  an element from a value the page computes: a bar's height, an
  avatar's colour. The property is written after the colon, as a
  literal; `expr` is a signal read, a memo or a plain value of the type
  the property fixes: a px `f32` for `width`, `height`, `top`, `left`,
  `margin-*` and `padding-*`, a `vapors_elements::Rgba` for
  `background-color` and `color`. The binding wins over the sheet, as
  an inline style does, and each property is bound once. Elements that
  differ only in a bound value share one style record (1,000 bars of
  12 heights are 12 styles); a changed value restyles that element
  alone and asks layout only what the property needs (a colour, none).
  A value outside the property's domain (NaN or infinity anywhere, a
  negative `width`, `height` or `padding-*`) is refused loudly naming
  the element, never clamped, and the element keeps its last valid
  value. A binding is for elements, not component tags. Decision:
  `docs/decisions/2026-11-20-style-bindings.md`.
- **Bindings:** `bind:value={signal}` on `input`, `textarea` and
  `select`, and `bind:checked={signal}` on a checkbox or radio. The
  signal sets the property, and the control's event (`input`, or
  `change` for `select` and `checked`) sets the signal. On an `input`
  whose `type` is `range` or `number` the value is a number
  (`Signal<f64>`, `event.value_as_number()`), elsewhere text, so the
  `type` is written as a literal; a number field's binding updates on
  `change` (Enter, blur, a step), since its text is a number only once
  committed. A number set on a number field shows at its `step`
  attribute's precision (0.5000001 by 0.01 shows "0.50"), while its
  value keeps the full number; Chromium shows the full number
  (`crates/elements/src/number_shown.rs`). A binding plus a handler for
  the same event is refused.
- **Conditionals:** `{if cond} .. {else} .. {/if}`. There is no
  `{else if}`. Nest an `{if}` inside the `{else}` instead.
- **Loops:** `{for (i, x) in expr key=expr} .. {/for}` or
  `{for x in expr key=expr} .. {/for}`. Without `key=`, the index is
  the key.
- **Lifetime:** a component mounted from Rust lives as long as its
  struct. Dropping it disposes its signals and effects (its nodes stay
  in the document until `unmount` removes them), so keep the struct
  while the page shows; the struct is `#[must_use]`, and a mount whose
  result is dropped at once does not compile under `-D warnings`.
  `let _ = Page::mount(..)` compiles: it is an explicit discard, and
  disposes the component at the end of the statement (its nodes drawn
  once, then never updated); bind a name (`let _page = ..`) to keep it. A
  child component's struct is dropped inside its parent's mount: the
  parent's generated code calls `owned_by_parent()` on it, its owner is
  a child of the parent's, and it is disposed with the parent (#353).
- **Child components:** a capitalised tag, `<TodoItem label={x} />`, or
  a path, `<forms::TextInput />`. The tag must name the type, and the
  same name with `Props` appended must name its props struct, so a
  plain tag needs a `use` in the script. Attributes become props, and unset props take their
  defaults. `bind:value={s}` on a component passes the signal itself.
  `class`, `class:name={bool}` and `style` on a component tag are not
  props (`style:property={expr}` is refused there): the tag gets a style key as an element does, passed as the
  generated prop `anchor: Some(&KEY)`, with the tag its props type
  declares (`CardProps::ANCHOR_TAG`), and the component creates its
  one anchor element from it (`Dom::create_keyed_element`); the
  directives toggle the element its `anchor_element()` returns
  (`docs/plans/css-for-3d.md`, C1). The 3D scene components take
  them. Only a Rust component can have an
  anchor: on a `.wisp` component in the same directory they are
  refused at each attribute, and another crate's component without the
  field fails in rustc at its props struct, naming `anchor`. Writing
  `anchor={..}` on a tag is refused.
- **Unchecked props:** a tag naming another crate's component compiles
  with its props unchecked (rustc checks them in the generated file).
  The build prints one warning per run, naming each such component once
  with its count; `WISP_NOTES=1` adds each tag's `file:line:col` after it.
- **Children:** markup inside a component tag is the child's
  `children` prop, a `Snippet`. The child declares it, `#[prop] let
  children: Snippet = Snippet::empty();`, and places it with
  `{render children()}`. A tag passing a prop its component does not
  declare, children included, is refused when the component is in the
  same directory.
- **Snippets:** `{snippet name(param: Type, ...)} .. {/snippet}`
  declares a piece of template, and `{render name(args)}` builds it in
  place. A snippet is visible in the list it is declared in and in the
  lists inside it, and may be rendered before it is declared or by a
  snippet declared before it. A snippet declared directly inside a
  component tag is the child's prop of that name (a `Snippet<(Type,
  ...)>`). A local snippet passed by name in an attribute, `<List
  row={b} />`, is passed as a clone, so `b` can still be rendered.
- **Rendering an expression:** `{render expr(args)}`, where `expr` is
  anything that yields a `Snippet`, an `Option<Snippet>` (`None`
  renders nothing) or a reference to either: `cards.get(&kind)`,
  `(if a { x.clone() } else { y.clone() })`, or the name of a signal or
  memo holding one. It is read in an effect, and the snippet shown is
  rebuilt when `expr` yields a different snippet. Snippets compare by
  identity (clones of one snippet are equal; two built alike are not),
  so create snippets once: one made inside the expression, on every
  read, is rebuilt on every change of what the expression reads.
  By name, in place, are local snippets and props declared `Snippet` or
  `Snippet<..>`; anything else, an `Option<Snippet>` prop included,
  takes the expression route, where rustc checks the type against
  `Renderable` (in the generated file). `None` always renders nothing,
  silently: there is no way to say a snippet was expected.
- **Snippet parameters are reactive.** The render passes its arguments
  as a closure, and each parameter is a `Memo` of its argument, so a
  snippet follows its arguments as they change: a kept `{for}` row whose
  item changes updates its snippet in place. A parameter reads as its
  value in a hole and in a render's arguments, and as the memo in
  attributes and conditions (`name.get()`), as a loop binding does. A
  parameter type must be owned (`String`, not `&str`), `PartialEq` and
  `Clone`; a `Signal<..>` or `Memo<..>` parameter is refused, since the
  parameter is reactive already. Recursion, direct or through another
  snippet, is refused for now.
- **A component inside a snippet.** A snippet's body is built against
  `dyn Dom`, and a component tag in it is mounted there: a `.wisp`
  component of the same directory as `Name::mount_before::<dyn Dom>`,
  any other tag with the plain call, as a hand-written mount (the scene
  tags' `&impl DynDom`) needs. A `.wisp` component of another crate
  inside a snippet does not compile yet (rustc reports mismatched
  types at its `mount_before`): the compiler cannot tell it from a
  hand-written one until it reads another crate's components.

## Page-level keys: `<vapors:window>`

`onkeydown` and `onkeyup` on an element hear only the keys that element
has focus for. A game reads the arrow keys whatever has focus, so a
component takes them for the whole page with `<vapors:window>`, as
Svelte's `<svelte:window>` does:

```
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

<p>{x}</p>
<vapors:window onkeydown={step} />
```

The handlers are the ones an element's key listeners take (`&mut Event`
or none), so code moves between the two unchanged.

- Every key event the page gets reaches each mounted `<vapors:window>`,
  after the listeners on the focused element (or the body) and its
  ancestors ran. An event one of them stopped with `stop_propagation`
  does not reach the window.
- The listeners are removed when the component unmounts.
- It goes at the top level of the template: not inside an element,
  `{if}`, `{for}`, a snippet or a component tag. One per component, self-closing,
  with `onkeydown` and `onkeyup` as its only attributes, each an
  expression. Anything else, and any other `vapors:` tag, is refused with
  its line and column.
- A game calls `event.prevent_default()` for the keys it uses. That marks
  the key used by the document (`Document::key_pressed` answers `true`),
  so the app does not also act on it, and stops the element defaults
  (Tab's focus move, a range's step). It does not reach the browser:
  whether the arrow keys or space also scroll a web page is the
  platform's, not the template's, and a canvas page usually has nothing
  to scroll.

## Reactive rules

- Each `{expr}` hole is its own text node with its own effect. When a
  signal the expression reads changes, that effect sets that one text
  node. Nothing else in the tree is touched. A hole that reads no signal
  is set once.
- In a text hole, a bare signal or memo name is read for you:
  `{count}` means `count.get()`, and `{items.len()}` means
  `items.get().len()`. An explicit `{count.get()}` is left alone, as is
  any name that is not a signal, memo or signal-typed prop.
- Attribute expressions, `{if}` conditions and `{for}` iterables are
  used as written, so they say `.get()` themselves:
  `title={label.get()}`, `{if count.get() > 0}`.
- An expression attribute is its own effect, which sets that attribute
  when its signals change. A literal attribute is set once.
- `{if}` keeps one branch built before an anchor node. It rebuilds only
  when the condition changes value. Each branch has its own owner,
  which is disposed when the branch is removed.
- A keyed `{for}` matches rows by key across runs. A kept row keeps its
  nodes, and its index and item signals are updated in place. A reorder
  moves only the rows that are not on the longest increasing run of old
  positions. A removed row is disposed. The todo-list example checks
  that a reorder makes only `insert_before` calls.
- Two rows with the same key are refused when the items change. The
  message names the key and the `{for}`'s `file:line:col`. The check
  runs before any row is touched, so the rows stay as they were and
  the next update applies normally.
- A block at the top of a branch, a row or a component belongs to it.
  Removing or moving the branch, row or component removes or moves the
  inner block's current nodes too. A component's `roots()` lists them.
- A prop is read once at mount unless its type is `Signal<T>`, in which
  case it reads like a signal. A child's props are not re-sent when the
  parent's values change.
- A computed prop: `transform={Signal::derived(move || tumble(time.get()))}`,
  a read-only `ReadSignal<T>` (see "Read-only props"). It follows what the closure reads and belongs to the owner current at
  the call, so in a `{for}` row it ends with the row.
- Signals follow Leptos's semantics. Effects run after the write, or
  after the outermost `batch`, in creation order. A diamond runs once.
  An effect's first run follows the same rule: a signal it writes is
  picked up after the run ends. Reading a disposed signal, or a memo
  cycle, is refused with the node's name. A panic in user code leaves
  the runtime usable once it is caught. See the `vapors-signals` crate
  docs.

## Time and animation

CSS transitions and animations need no clock: write them in `<style>`
and the app draws the frames they need. Use a clock when the script
must know the value itself, as a game does to test a collision.

```html
<script>
use vapors_elements::frame_time;
let x: ReadSignal<u128> = Signal::derived(move || frame_time().get().as_millis());
</script>

<p>{x}</p>
```

`vapors_elements::frame_time()` is the time since the first frame that
followed the first call, and `frame_delta()` the time since the
previous frame (zero on the first). Both are `ReadSignal<Duration>`, one
pair for the whole app: every call returns views of the same signals, set once
at the start of each frame from the frame's own time, so two readers
see one clock. After a wait (nothing changed, so no frames) the
`frame_delta` includes the wait.

### The viewport

`vapors_elements::viewport_size()` is a `ReadSignal<Size>` (`width` and
`height` in CSS px) of the drawing area: the canvas on the web, the
window's content natively, the numbers layout and `@media` use.
`orientation()` is a `ReadSignal<Orientation>`, `Portrait` when taller
than wide and `Landscape` otherwise (a square is landscape), the rule of
`@media (orientation)`. Both are app-wide, zero or landscape until the
first update, and set by the app only when the size changes: reading them
never asks for a frame.

```html
<script>
use vapors_elements::{orientation, Orientation};
let fov = Signal::derived(move || match orientation().get() {
    Orientation::Portrait => 80.0,
    Orientation::Landscape => 55.0,
});
</script>
```

While an effect, a derived signal or a template binding that reads
either signal is mounted, the app draws every display refresh
(`requestAnimationFrame` on the web); when the last one unmounts it
goes back to drawing on change. There is no start or stop call, and a
page that never calls them draws as before. A derived signal nothing
reads still counts as a reader until its owner is disposed.

An element's style cannot follow a signal yet: `style={..}` is refused
(`docs/decisions/2026-11-11-analysable-sources.md`) and a typed
per-property binding is still being decided. Today a 3D scene's
`transform={Signal::derived(move || ..)}` on a scene tag is the way to
move things every frame (`docs/scene3d.md`).

## Required props

A `#[prop]` declared without a default is required: the parent must pass
it, as when a default would be wrong (a shared `Controls`, where a
private copy would split the state).

- A tag in the same crate that omits it is refused at the tag, naming
  the file and line, the component and the prop, and where the
  component declares it.
- A tag the compiler cannot check (a component of another crate: its
  props are unchecked) reaches the mount, which panics naming the
  component, the prop and its declaration; it never fills a default.
  `Required::take` (`vapors_dom::Required`) does it.
- The field is `Required<T>`, `Missing` in `Default`. A checked tag
  wraps what it passes in `Required::Given`; Rust code that builds the
  props struct writes `Required::Given(v)` (or `v.into()`). A tag the
  compiler cannot check does not wrap, so it cannot pass a required
  prop yet: that stays a rustc error at the props struct, until the
  compiler reads another crate's props (the unchecked-props gap above).
- The script reads the prop as its plain type `T`, as any prop.
- A prop with no type is still refused.

## Lints

A generated `mount_before` takes its props by value and moves them
whole into a one-element tuple before destructuring, so
`clippy::needless_pass_by_value` never fires, whatever the props' types
(`Copy` ones included): no component needs an allowance of its own.

## Read-only props

A prop typed `ReadSignal<T>` gives the child read access only: it has
`get`, `with`, the `_untracked` and `try_` forms, and no `set` or
`update`. A prop typed `Signal<T>` is a deliberate grant of write
access, as Svelte's `$bindable` is: the child may write it, and the
parent's `bind:name={signal}` passes the signal itself. In Svelte a
prop is read-only unless declared `$bindable`; here the type says it.

The scene tags' `transform` and `projection` props take `ReadSignal`.
A `Signal` or `Memo` binding of the script passed alone converts, as
in `transform={tilt}` (the compiler writes `.into()` for a binding it
knows is a signal or memo); `Signal::derived(..)` and `frame_time()`
already are read-only. Any other expression, such as a field of a
struct or a function's result, calls `.read_only()` itself
(`signal.read_only()`) or returns a `ReadSignal`. Computed values are
read-only because a write would be lost at the next change of their
inputs: `Signal::derived(..).set(..)` does not compile.
(`docs/decisions/2026-11-18-read-signal.md`.)

## Refusals

Every refusal is printed as `file:line:col: message` (the build adds
rustc's `error: ` in front), with 1-based
lines and columns counted in characters. The message names the fix.
The compiler refuses:

- section problems: a second `<script>` or `<style>`, an unclosed
  section, attributes on a section tag, a section after the template, a
  file with no template;
- unknown lowercase tags, with the capitalised form suggested;
  mismatched or unclosed tags, naming where the tag opened; a close tag
  for a void element;
- bare or single-quoted attribute values, repeated attributes, and an
  event attribute with a string value;
- unterminated, empty or unbalanced `{}` holes, and a stray `}`;
- `{else if}`, an `{else}` or `{/..}` with no open block, and a `{for}`
  pattern other than `x` or `(i, x)`;
- HTML comments, `<title>`, and sumi's slots, each with a pointer to
  what to use instead;
- Rust syntax errors in the script or in any expression, positioned in
  the `.wisp`. An expression that ends too early is positioned where it
  ends.
- a signal or memo with no type annotation; a `#[prop]` with no type,
  no default, or a pattern in place of a single name;
- a script, prop or loop binding named `dom`, `parent`, `before`,
  `props`, `owner`, `roots`, `event` or `snippet_args`, which the generated code uses,
  or named like a node the generated code creates for this template
  (`p0`, `button3`); and a raw identifier such as `r#gen`;
- an unknown character reference, and a Rust if-expression in a hole;
- an HTML boolean attribute (`checked`, `disabled`, `open`, ...) from
  an expression, which would be on whatever its value, since generated
  code cannot remove an attribute: use `bind:checked`, or the attribute
  on an element inside `{if}`;
- `class={..}` and `style={..}` on an element or a component tag: the compiler must see
  every class and declaration an element can have, so a class is
  literal (`class="name"`) or toggled (`class:name={condition}`), and
  declarations go in the `<style>` section
  (`docs/decisions/2026-11-11-analysable-sources.md`);
- an element attribute named with `:` other than the `bind:`, `class:`
  and `style:` directives (`on:click` included; a listener is
  `onclick`), and a `class:` directive with a quoted value or a
  name other than letters, digits, `-` and `_`;
- a `style:` binding of a property that is not bindable (the message
  lists those that are), with a computed name (`style:{name}`), on a
  component tag, with a quoted value or a literal of a type the property
  cannot take (`style:height={4}`, an integer), or the same property
  twice on one element;
- `bind:` on an element that is not a control, on a property other than
  `value` or `checked`, or together with a handler for the same event;
- a `data-wisp` attribute, which is reserved for the style scope (see
  "Scoped styles");
- snippets: a parameter without a type, with a pattern for a name,
  that borrows, that is a `Signal<..>` or `Memo<..>`, or that has the
  name of a script binding (both lines named); two snippets of one name
  in a list; a snippet that renders itself; a `{render}` of nothing
  declared where it is, or with a different number of arguments from
  the snippet's parameters.

To check one file without a build, run:

```
cargo run -p vapors-compiler --bin wisp-check -- path/to/file.wisp
```

It prints `ok: <file> compiles to component `Name`` and exits 0, or
prints the refusal and exits 1. It runs code generation as well as
parsing, so it reports the same refusals as the build. It is a native
binary: its wasm build panics, saying so.

## Wiring it into a crate

```toml
# Cargo.toml
[dependencies]
vapors-elements = { path = "../vapors/crates/elements" }
vapors-signals = { path = "../vapors/crates/signals" }

[build-dependencies]
vapors-compiler = { path = "../vapors/crates/compiler" }
```

```rust
// build.rs
fn main() {
    vapors_compiler::build::compile_dir("src");
}

// src/lib.rs
include!(concat!(env!("OUT_DIR"), "/wisp.rs"));
```

`compile_dir` compiles every `.wisp` under `src` into
`OUT_DIR/wisp/`. It then writes `OUT_DIR/wisp.rs` with one documented
`pub mod` per component, named after the file in snake case.
`src/todo_list.wisp` becomes `todo_list::TodoList` and
`todo_list::TodoListProps`. A subdirectory that holds `.wisp` files
becomes a nested module.

- A relative directory is resolved against `CARGO_MANIFEST_DIR`, so the
  paths in refusals open from wherever cargo was run.
- It prints `cargo::rerun-if-changed` for each scanned directory and
  each `.wisp` file. Watching the directories means a new file triggers
  a rebuild. It also means that editing any file in `src` reruns the
  build script.
- On a refusal, every refusal from every file is printed to stderr as
  `error: file:line:col: message` and the script exits with status 1.
  Nothing is written.
- It also refuses: a directory with no `.wisp` files; two files or
  directories that would make the same module, such as
  `text-input.wisp` and `text_input.wisp`; a directory name that is not
  a Rust module name; a file or directory whose name makes a Rust
  keyword, such as `type.wisp`; and a missing `OUT_DIR`, which happens
  when it is called outside `build.rs`.

A page's crate needs `vapors-elements` and `vapors-signals`, not
`vapors-dom`: generated code names the dom through the hidden
`vapors_elements::__wisp::dom` (the file's second line says which way).
A crate below `vapors-elements` cannot have it: the layers rule
(`.config/factory.toml`) forbids `crates/components` depending on
`vapors-elements`, and `crates/compiler-examples` has it only for tests.
Those two call `vapors_compiler::build::compile_dir_below_elements("src")`
in `build.rs`, which names `vapors_dom::` as before and needs
`vapors-dom` as a dependency. Your own script may still name
`vapors_dom` if the crate depends on it. `crates/wisp-page` builds
without it.

A script's `use` items are emitted at the top of the generated module,
so a `#[prop]` type may name an imported item. A `use` of an item the
script declares itself (`enum Mode {..}` then `use Mode::*;`) cannot
move there and is refused; `use` it from a module of the crate.

## Reading a rustc error in a page

The compiler's refusals point into the `.wisp`; rustc's errors in your
script code point at `target/.../out/wisp/<page>.rs`. Run
`cargo run -p vapors-compiler --bin wisp-check -- path/to/page.wisp`
first: it reports every refusal without a build. For a rustc error,
open the generated file at the line it names. Each piece of script
code (a `use`, a statement) follows a comment such as
`// from pages/spin_cube.wisp:12`, and a prop's doc says `at
pages/spin_cube.wisp:12`; the nearest one above the error is the line
to edit. Template expressions are not marked: their code sits under the
`// from` of the last script statement, so look for the expression text.

## The generated code

For `counter.wisp` the compiler writes:

- `pub const STYLE: &str`, which holds the `<style>` text unchanged;
- with a `<style>`, `pub const SCOPE: &str`, the scope its rules are
  limited to (see "Scoped styles");
- `CounterProps`, with one field per `#[prop]` and a `Default` impl;
- `Counter`, with one public field per signal or memo, plus its owner
  and root nodes;
- `Counter::mount(dom, parent, props)` and `mount_before(dom, parent,
  before, props)`, which run the script under the component's `Owner`
  and build the template through `Dom`;
- `roots()`, the top-level nodes as they are now, and `unmount(dom)`,
  which reads them, disposes the owner, so its effects stop, and
  removes them.

The file is included into the user's crate, where clippy pedantic runs
on it. The compiler reads syntax, not types. A few lints therefore
depend on facts it cannot see, such as whether a captured binding is
`Copy`, or whether a text hole's value is already a `String`. Each
such lint is allowed on the one generated `let` that trips it, with
its reason in the attribute. A hole's expression is bound by its own
statement first, so the allowance never covers code the user wrote.
Lints in the user's script and expressions are still reported. One
allowance sits on `mount_before` because it judges the function
itself: its length, which follows the template's size; props taken by
value need none (see "Lints" above). Generated node names need none: each is
a stem and a counter, and the counters share one width per component
(`text03` and `text13` in a template of more than ten names), so
clippy's `similar_names` never sees two that differ by an inserted
digit. The list and the reasons are in
`crates/compiler/src/codegen/allowances.rs`.

## Scoped styles

A component's `<style>` applies only to the elements that component
creates; the rule is Svelte 5's.

- `SCOPE` is the crate's name, a `/`, and the file's path under the
  directory `compile_dir` was given, counted from that directory's
  parent: `compile_dir("src")` in `vapors-compiler-examples` gives
  `vapors-compiler-examples/src/counter.wisp`. It is the same on every
  machine.
- `mount_before` first calls `dom.add_style(SCOPE, STYLE)`. A `Dom`
  adds a scope once, however often the component is mounted.
- Each element the component creates gets
  `data-wisp="<SCOPE>"` (`vapors_dom::SCOPE_ATTRIBUTE`). A child
  component's elements get the child's scope, not the parent's.
- vapors-elements rewrites each selector so that every compound
  requires the scope. The subject compound (the last) requires it as an
  attribute selector. Every other compound, and every compound inside
  `:is()`, `:not()`, `:where()` and `:has()`, requires it inside
  `:where()`, which adds no specificity. A scoped rule therefore gains
  exactly one attribute's specificity, (0,1,0), however many compounds
  it has. When the subject is `:global(..)`, the last scoped compound
  counts instead: `.card :global(p)` weighs (0,2,1).

For example, in a component whose scope is `app/src/panel.wisp`:

```css
/* written */
.panel .title:hover { color: red; }

/* as matched */
.panel:where([data-wisp="app/src/panel.wisp"])
  .title:hover[data-wisp="app/src/panel.wisp"] { color: red; }
/* specificity (0,4,0): .panel, .title, :hover, and the scope once */
```

- `:global(...)` leaves the compound holding it unscoped, for an
  element the component did not create: `:global(body) { margin: 0; }`,
  or `:global(.dark) .title` for `.title` under any `.dark`. It weighs
  what its argument weighs. Only a top-level compound can be left
  unscoped: a `:global` inside `:is()`, `:not()`, `:where()` or `:has()`
  is refused, naming the line. In a sheet added with `add_stylesheet`,
  which has no scope, `:global` is refused anywhere.
- A scope added again with a different style (two components with the
  same scope) is refused, naming the scope.
- A style that does not parse, or holds a `:global()` the scoping
  cannot honour, fails the build (`compile_dir` and `wisp-check`),
  naming `file:line:col`; the compiler reads it with `vapors-css`, the
  runtime's own parser and scoping (`vapors_css::scope`), so the two
  give the same parse and `:global()` refusals (the runtime's
  per-element value refusals still happen at run time). A style the runtime is given by hand
  (`add_style`) is refused by the next update, naming the scope, line
  and column.
- `wisp-check --styles <file>` prints the build-time styles analysis
  (`docs/plans/build-time-styles.md`, section 1): per element, each
  candidate rule as never, always or conditional with what it depends
  on, and per declared property its winner and whether the computed
  value is constant.
- A crate whose components' styles the app adds as global sheets uses
  `compile_dir_global` instead. Nothing is then scoped, and the
  generated code has no `SCOPE`, `add_style` or tags. The layout corpus
  does this, so its CSS means what Chromium reads.
- Not supported: `@keyframes` names are global; `:global { ... }`
  blocks and `:global` inside `:is()` are not supported.

## Deferred

- **CSS values.** The compiler parses `STYLE` and refuses what does not
  parse; it does not yet classify which rules match which elements
  (`docs/plans/build-time-styles.md`, B1 onward).
- **Elements (F2c).** The only `Dom` implementation is the recording
  fake in `vapors_dom::recording`, used by tests. The elements crate
  implements `Dom` over the real element tree, with layout and paint.
- **Accessibility (F2d).** Implicit roles, ARIA attributes and the
  AccessKit tree are not generated or checked.
- **In the compiler:** recursive snippets; another crate's `.wisp`
  component inside a snippet (see "A component inside a snippet"). Props passed to a child are
  not reactive unless they are `Signal`s. There is no hot reload, LSP or inspector yet.
