# How the apps are written

gui-bench compares frameworks the way an app author meets them. Every app
follows one rule, and this file lists where an app departs from it.

## The rule

1. **Built-in parts.** An app uses its framework's own scroll container, text
   input, checkbox, switch, button, menu, virtual list, timer and animation.
2. **The documented pattern.** State and views are structured the way the
   framework's documentation and examples structure an app of this kind:
   signals per value, components per part, entities per view, or one `view`
   function, as the framework teaches.
3. **Defaults.** Renderer, present mode, scroll scale, scroll smoothing,
   momentum and scrollbars stay as the framework ships them. No app rewrites
   scroll or key input.
4. **No app-side optimization.** No manual caching, memoization,
   virtualization or skipped work beyond what rule 2 covers. Where a framework
   offers an opt-in cache that its documentation describes, the plain app
   leaves it out and a `tuned` edition uses it, so results show both.
5. **Missing parts are built by hand and flagged.** Where a framework has no
   part of a kind, or its part does not work at the pinned version, the app
   builds the smallest straightforward one. That is what an author of such an
   app would have to do, so it is the idiomatic code for that framework. The
   runner records each such part in `run.json` (`builds.<framework>.flags`),
   and the plots draw its measurements with hollow markers.

## Tuned editions

| Edition | The plain app | The tuned edition |
|---|---|---|
| `gpui-kit-tuned` | embeds every view entity plainly; a notified entity re-renders its ancestors and their other children | embeds the sidebar, the top bar, the metric cards and the orders with `cached`, so GPUI reuses their last frame while they are not notified |
| `iced-tuned` | builds every widget in `view` on every message | wraps the orders card, the message feed and the rows of the list in `lazy`, keyed by the data they show |

Both are one cargo feature (`tuned`) around one helper function per app.

## Parts built by hand

Flagged parts carry their name in `run.json`.

| Framework | Part | Flag | What the app does, and why |
|---|---|---|---|
| egui | switch | `switch` | egui has no switch. The compare and live switches are the `toggle_switch` of the egui demo. |
| egui | spinner, live timer | – | Immediate mode has no animation or timer objects: the angle and the tick come from the input time, with `request_repaint` and `request_repaint_after`, as egui's own `Spinner` does. |
| egui | font weights | – | egui has no weights. Semibold and bold are font families of their own. |
| egui | sorted data | – | The sorted orders live in the app state and are sorted again when the sorting toggles, not on every pass. |
| iced | virtual list | `list` | iced has none. `on_scroll` gives the viewport; `view` builds the visible rows and 4 more on each side between two spaces. |
| iced | menu | `menu` | iced has no menu or popover (`pick_list` is a select as wide as its button). The menu is a container of buttons, pinned in a `stack` over the window. |
| iced | spinner | `spinner` | Widgets have no transform animation. A `canvas` draws the turned square on every frame of `window::frames`. |
| iced | hover color | – | Containers have no hover style. Order rows, nav items and menu items are buttons whose style follows the hover status. |
| iced | switch size | – | A `toggler` is twice as wide as tall: 44x22 in place of 40x22. |
| freya | menu | `menu` | `Menu` has no anchor. The app places it in an absolute box under the button, as freya's `SubMenu` places its own. |
| freya | button shadow | – | The button theme has no shadow. A box around the primary buttons carries it. |
| gpui-kit | spinner | `spinner` | GPUI rotates only `svg` elements. The square is an inline SVG under `with_animation`, as in gpui-component's `Spinner`. |
| vizia | spinner | `spinner` | Animations cannot repeat without end at 0.4.0. A 16 ms timer sets the angle, so the spinner turns at about 62 Hz at any refresh rate. |
| vizia | row rules | – | Vizia has no border per side. The rule under a row is a 1 px gradient background. |
| vizia | line height | – | No `line-height` property. Message lines take the font's height, about 19 px. |
| floem | menu | `menu` | Floem has no styleable menu. The app adds a stack of items with `add_overlay` under the button, as Floem's `Dropdown` does; the declarative `Overlay` view keeps the window from settling at the pinned revision. |
| floem | spinner | `spinner` | A repeating keyframe animation stops after one pass at the pinned revision. The app turns an angle signal from `exec_after_animation_frame` and paints the square on a `canvas`, since vger draws a rotated rounded rect as its bounding box. |
| dioxus | virtual list | `list` | Dioxus has none. `onscroll` gives the viewport; the component renders the visible rows and 8 more on each side between two spacers. |
| dioxus | switch, menu | – | Written as on the web: a `button role=switch`, and a conditionally rendered absolute box for the menu. |
| dioxus | placeholder | – | Blitz does not paint `placeholder`. A span over the input shows it. |
| lorelei | switch, checkbox | `switch` | Lorelei has no component library. A switch is a track with a knob, a checkbox a box with a mark, each a row of boxes with a click route. |
| lorelei | menu | `menu` | Lorelei has no menu or popover. The menu is a box of items at an absolute spot under the button, the last child of the window. |
| lorelei | spinner, live timer | `spinner` | The application surface (`bough::prelude`) has no timer, no rotation and no repeating animation. The app takes the frame callback of the host (`Run::linked`): while live is on, every refresh runs one turn that sets the tick and the angle from the time. The square is an inline SVG that the scene-level `svg_image` turns. |
| lorelei | font | `font` | Lorelei panics at its first text with a font whose Latin shaping it cannot put in a table, and Noto Sans, the `sans-serif` of this host, is such a font. The app takes the first of the system `sans-serif`, Cantarell, DejaVu Sans, Arial, Helvetica and Verdana that Lorelei can shape: here Cantarell. |
| lorelei | reclaim | `reclaim` | Lorelei returns the elements that left the tree to its free lists a few thousand at a time, in every turn and in every frame without damage, and its Wayland host runs neither after input. Without help every replaced page keeps its glyphs, and the app panics when the glyph column (4,194,304 slots) is full: after about 18 page switches with 3,000 rows, 60 with 1,000 and 210 with 300. The app runs empty turns (`App::poll`) after a frame until nothing waits. |
| lorelei | window | – | `bough::backend::run`, the entry below `Application::run`, with the configuration that `Application::run` gives a window: only it takes the application id and the frame callback. |
| lorelei | content width | – | A scroll box lays its content out at the width of the window, whatever its own width. The content of the app leaves the width of the sidebar free at its right. |
| lorelei | weights, shadows, corners | – | The application surface has one font face, no shadow and one radius per box. All text is regular, the cards, buttons and knobs are flat, and the bars and the header of the orders round all four corners. |
| lorelei | clipping | – | Only scroll boxes clip what they hold. In a narrow window the bars and rows that do not fit their card are painted beside it. |
| lorelei | button, text input | – | `button` and `text_input` have one look each. The buttons are rows styled the way `button` is built. The search input is `text_input` as it is (white, radius 6), with the placeholder as a text over it. |
| lorelei | nav items | – | A hover color is fixed when its element is mounted. The active and the plain nav item are two elements, of which one exists. |
| lorelei | list | – | `list` shows a `List` of one index per row, and `scrolls` takes the scroll range, which the app computes from the row count and the height of the window. |
| tauri | data | – | The page asks the Rust side for the spec's data with one command at startup. The list computes its rows in TypeScript from the same tables, since a million rows do not cross the IPC. |
| tauri | virtual list | – | `@tanstack/solid-virtual`, the virtual list of the Solid ecosystem; SolidJS has none of its own. |
| tauri | switch, menu | – | Written as on the web: a `button role=switch`, and a conditionally rendered absolute box for the menu. |
| tauri | window | – | No decorations, so the page starts at the top of the window. |
| tauri | renderer | `renderer` | The app sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` unless the environment sets it. With the DMA-BUF renderer, WebKitGTK on Wayland with the NVIDIA driver presents 60 frames per second on a 144 Hz output; turning it off there is common practice for Tauri apps. This host is that worst case for Tauri: other platforms need no such switch. |
| zgui | – | – | None. |

## Known defects that the apps do not work around

- **Dioxus Native, vello-hybrid:** a CSS animation starts only after an input
  asks for a redraw. The live switch click is such an input.
- **Tauri:** WebKit caps the height of an element at 33,554,432 px, so of
  1,000,000 rows of 52 px only the first 645,000 can be reached.
- **Vizia:** the first frame of the window is empty; the content follows in a
  later frame. The runner takes the startup time to the frame with the
  content (`content_ms`).
