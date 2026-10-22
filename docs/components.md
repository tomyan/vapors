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
| `let increment = move \|\| ..;` | a handler. A closure with no parameters is wrapped so it can take an event. |
| `use crate::todo_item::{TodoItem, TodoItemProps};` | brings a child component into scope for its tag |

A signal or memo needs its type written out, because it becomes a struct
field. `let count = Signal::new(0)` is refused, and the message gives
the annotated form. The brief's examples omit the annotation. That is
the one difference from the brief in this slice.

## Template

- **Elements** come from a fixed HTML vocabulary, listed in
  `crates/compiler/src/template/vocabulary.rs`. Void elements such as
  `input` and `br` close themselves. Any other lowercase tag is refused.
- **Text** may hold `{expr}` holes. A whitespace-only run that contains
  a newline is formatting and produces nothing. One on a single line
  produces one space. This is the JSX rule.
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
  Each name may appear once.
- **Events:** `onclick={handler}` listens for `click`. The event name
  is the attribute name after `on`. The handler is a script closure or
  an inline closure that takes `&mut Event`. Events bubble.
  `event.stop_propagation()` and `event.prevent_default()` work as in
  the DOM.
- **Bindings:** `bind:value={signal}` on `input`, `textarea` and
  `select`, and `bind:checked={signal}` on a checkbox or radio. The
  signal sets the property, and the control's event (`input`, or
  `change` for `select` and `checked`) sets the signal. A binding plus
  a handler for the same event is refused.
- **Conditionals:** `{if cond} .. {else} .. {/if}`. There is no
  `{else if}`. Nest an `{if}` inside the `{else}` instead.
- **Loops:** `{for (i, x) in expr key=expr} .. {/for}` or
  `{for x in expr key=expr} .. {/for}`. Without `key=`, the index is
  the key.
- **Child components:** a capitalised tag, `<TodoItem label={x} />`, or
  a path, `<forms::TextInput />`. The tag must name the type, and the
  same name with `Props` appended must name its props struct, so a
  plain tag needs a `use` in the script. Attributes become props, and unset props take their
  defaults. `bind:value={s}` on a component passes the signal itself.

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
  `class={theme.get()}`, `{if count.get() > 0}`.
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
- Signals follow Leptos's semantics. Effects run after the write, or
  after the outermost `batch`, in creation order. A diamond runs once.
  An effect's first run follows the same rule: a signal it writes is
  picked up after the run ends. Reading a disposed signal, or a memo
  cycle, is refused with the node's name. A panic in user code leaves
  the runtime usable once it is caught. See the `vapors-signals` crate
  docs.

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
  `props`, `owner`, `roots` or `event`, which the generated code uses,
  or named like a node the generated code creates for this template
  (`p0`, `button3`); and a raw identifier such as `r#gen`;
- an unknown character reference, and a Rust if-expression in a hole;
- `bind:` on an element that is not a control, on a property other than
  `value` or `checked`, or together with a handler for the same event;
- deferred features: `{snippet}`/`{render}`, which parse but do not
  generate code, and children inside a component tag.

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
vapors-dom = { path = "../vapors/crates/dom" }
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

## The generated code

For `counter.wisp` the compiler writes:

- `pub const STYLE: &str`, which holds the `<style>` text unchanged;
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
Lints in the user's script and expressions are still reported. Two
allowances sit on `mount_before` because they judge the function
itself: its length, which follows the template's size, and an empty
props struct taken by value. The list and the reasons are in
`crates/compiler/src/codegen/allowances.rs`.

## Deferred

- **CSS.** `STYLE` is the raw `<style>` text. The compiler does not
  parse it, so a CSS error appears at run time, not at build time. The
  `vapors-css` crate from F2a (`docs/css.md`) parses and cascades
  stylesheets. Connecting a component's `STYLE` to it, with sumi's
  per-component scoping, is not done yet.
- **Elements (F2c).** The only `Dom` implementation is the recording
  fake in `vapors_dom::recording`, used by tests. The elements crate
  implements `Dom` over the real element tree, with layout and paint.
- **Accessibility (F2d).** Implicit roles, ARIA attributes and the
  AccessKit tree are not generated or checked.
- **In the compiler:** snippets and `{render}` parse but are refused at
  code generation. Children inside a component tag are refused; they
  will become snippets. Props passed to a child are not reactive unless
  they are `Signal`s. There is no hot reload, LSP or inspector yet.
