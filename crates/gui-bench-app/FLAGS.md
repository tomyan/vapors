# Parts built by hand (`IDIOMS.md` rule 5)

Part, what vapors lacks, the smallest framework change. The window, the sidebar and the metric cards are plain elements and CSS.

| Part | What vapors lacks | The smallest change |
|---|---|---|
| search input placeholder | `<input placeholder>` is not painted or read: the compiler and the style engine have no placeholder. The top bar puts a `div` over the input, shown while the bound signal is empty, and `pointer-events: none`. | Paint `placeholder` of an empty text input (`::placeholder` colour) in `crates/elements`. |
| search input look | The user-agent sheet gives an input `overflow: hidden`, which vapors refuses with a border radius, and `display: inline-block`, laid out only as a flex item. The input sits in a flex box and sets `overflow: visible`, so a long text is not clipped to its rounded box. | Honour a rounded clip on an input (`docs/F2c.md` defers rounded clips). |
| scroll container axes | `overflow-y: auto` alone is refused (axes that differ). The content sets `overflow: auto` on both axes; it scrolls vertically only because nothing is wider. | Honour a one-axis scroll container. |
| sparkline bar heights | No `style=` and no `class={..}` with a computed name: the compiler must see every class. The 12 heights of the spec (even, 14 to 44) are 16 classes of the card, each toggled with `class:hN`. | A per-element `style:height={..}` directive that the style key admits. |
