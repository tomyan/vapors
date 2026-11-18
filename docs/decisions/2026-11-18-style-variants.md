# Style variants: typed class groups, and a component's supported styles

Date: 2026-11-18. Decided by: Tom, with the lead. Amends
`2026-11-11-analysable-sources.md` decision 1 ("chooses among a known
set with an enum"), which named the enum form but never designed it.
Closes the design question in #507.

## Decision

1. **A group of styles is declared in the stylesheet, and its Rust type
   is generated from it.** `@variants Name { Variant { ... } ... }`
   generates `enum Name { Variant, ... }` with the names exactly as
   written: no case change, no prefix. A variant exists only if its
   styles do, so rustc guarantees the mapping.
2. **A group is set through classes.** `class:Name={expr}` on an element
   applies the variant `expr` evaluates to; `expr` must have type
   `Name`. `class:name={bool}` keeps toggling one plain class. Each
   variant is a class under the hood, so specificity and the cascade
   are a class selector's.
3. **A component declares its supported styles with `#[style]`.**
   `#[style] let tone: Tone = Tone::Primary;` exports the group `Tone`,
   with a default (or `Option<Tone>` for none). A parent sets it with
   the same syntax, `<Button class:Tone={Tone::Danger}>`. The component
   binds the value where it applies (`class:Tone={tone}` on one or more
   of its elements). Props carry data and behaviour; styles carry
   appearance.
4. **Styles pass down by forwarding and by inheritance, never by
   reaching in.** A `#[style]` may have a child component's group type
   (`#[style] let tone: button::Tone`), forwarding it unchanged. Inherited
   properties and custom properties cross component boundaries as in
   CSS, so a variant that sets `--tone-fg` themes everything inside. A
   parent still cannot add rules to a child's elements.
5. **Ergonomics are part of the feature.** Every check below runs in
   `wisp-check` without cargo, positioned in the `.wisp` file and
   listing the valid options, so an agent's loop is write, check (about
   a second), fix, build.

## Example

```html
<!-- button.wisp -->
<style>
.btn { border-radius: 6px; background: var(--tone-bg); color: var(--tone-fg); }

@variants Tone {
  Primary   { --tone-bg: var(--accent); --tone-fg: #0a111b; &:hover { filter: brightness(1.1); } }
  Secondary { --tone-bg: transparent;   --tone-fg: var(--text); border: 1px solid var(--line); }
  Danger    { --tone-bg: #d03030;       --tone-fg: white; }
}
@variants Size {
  Small  { padding: 4px 10px; }
  Medium { padding: 8px 16px; }
}
@variants Press { Down { transform: translateY(1px); } }   /* internal */
.busy { opacity: .6; }
</style>

<script>
#[style] let tone: Tone = Tone::Primary;
#[style] let size: Size = Size::Medium;
#[prop] let busy: Signal<bool> = Signal::new(false);
let pressed: Signal<Option<Press>> = Signal::new(None);
</script>

<button class="btn" class:Tone={tone} class:Size={size}
        class:Press={pressed.get()} class:busy={busy.get()}
        onpointerdown={move |_| pressed.set(Some(Press::Down))}
        onpointerup={move |_| pressed.set(None)}>
  <slot />
</button>
```

```html
<!-- a parent -->
<Button class:Tone={Tone::Danger}>Delete</Button>
<Button class:Size={Size::Small}>Cancel</Button>

<!-- icon_button.wisp: forwards Button's tones -->
<script>
#[style] let tone: button::Tone = button::Tone::Primary;
</script>
<Button class:Tone={tone}><Icon name={icon} /><slot /></Button>
```

## Rules

- **Declaring.** `@variants` takes a Rust type name and variant names
  that are valid identifiers; each variant holds declarations and may
  nest rules (`&:hover`, `.icon`), which apply relative to the element
  the variant is bound to. A group is scoped to its component like any
  rule.
- **Binding.** `class:Name={expr}`: `Name` is a group of this component,
  or an exported group of the component the tag names. `expr` may be a
  literal path, a prop, a signal read, or a `match` from the app's own
  types; its type is the group, so the reachable set is the group's
  variants. `Option<Name>` binds the variant or none. A group is bound at
  most once per element.
- **Names in a parent.** In `class:Tone={...}` on `<Button>`, `Tone`
  names `Button`'s exported group, so both `Tone::Danger` and
  `button::Tone::Danger` are accepted. A script item that shadows a
  group's name is refused, naming the group.
- **Exporting.** `#[style]` types are this component's groups or a
  child component's exported groups (forwarding). A default is required
  unless the type is `Option<_>`. An exported style that no element binds
  is refused: it would do nothing.
- **Analysis.** An element's possible styles are the product of its
  bound groups' variants and its boolean toggles, read from the types.
  Winning declarations are computed per combination at build time; each
  directive is one effect.

## Refusals (build time, positioned in the `.wisp`, valid options listed)

| Mistake | Message names |
|---|---|
| `class:Tone={Tone::Ghost}` | the variants `Tone` has |
| `class:Tone={Size::Small}` | the expected group and the found type |
| `class:Colour={..}` on `<Button>` | the styles `Button` exports |
| `class:Press` on `<Button>` (internal group) | that `Press` is not exported, and the exported styles |
| the same group bound twice on one element | both positions |
| `#[style]` of a type that is not a group | the groups in scope |
| an exported style bound to no element | the style, and where a binding would go |
| a script `enum Tone` shadowing `@variants Tone` | both positions |
| `class:Name` where `Name` is both a group and a plain class | both declarations |
| a selector or `@variants` the parser refuses | as other style errors today |

`class={expr}` with a string stays refused, now naming three forms:
`class="literal"`, `class:name={bool}` and `class:Group={variant}`.

## Acceptance for agents

- `wisp-check` performs every check in the table without cargo.
- `wisp-check --api <file>` prints a component's props and styles, for
  example `style Tone = Primary | Secondary | Danger (default Primary)`.
- rustc errors from generated code are not the way a variant mistake is
  reported; if one is reachable, it is a defect against #508.
- `docs/styling.md` and the plugin skill carry the example above and
  the refusal table.

## Order

1. #508 first: errors in a page's Rust positioned in the `.wisp`, and the
   script's `use` items visible to generated code.
2. Run 1: `@variants` parsed, types generated, `class:Group` on
   elements with the checks and `wisp-check`, per-variant winners.
3. Run 2: `#[style]`, component tags, forwarding, `--api`, the docs.

## Open

- A group shared by unrelated components (a design-system `Tone` in its
  own style file, imported by each) is left until forwarding proves
  insufficient.
- Whether `class:name={bool}` later becomes a one-variant group is left
  open; both stay for now.
