<script>
// An editable combobox with a list of options, as the WAI-ARIA APG's
// "Editable Combobox With List Autocomplete" example behaves: a text
// field (role combobox) controlling a listbox shown while expanded, the
// active option named by aria-activedescendant. Its keys, while the
// field has focus:
// - ArrowDown/ArrowUp on a collapsed list fire `comboboxopen` at the
//   field; unless cancelled, the list opens on the first (last) option.
//   On an open list they move the active option, wrapping.
// - Home/End move the caret (the field's own) and leave no option active.
// - Enter with an option active fires `optionchoose` at the option;
//   unless cancelled, the field takes its text and the list closes. With
//   none active, the list closes.
// - Escape fires `comboboxdismiss` at the field; unless cancelled, an
//   open list closes, and on a closed one the field is cleared.
// Typing and filtering the options are the app's (it sets `options`).

#[prop]
let options: Signal<Vec<String>> = Signal::new(Vec::new());
#[prop]
let value: Signal<String> = Signal::new(String::new());
#[prop]
let expanded: Signal<bool> = Signal::new(false);
#[prop]
let active: Signal<Option<usize>> = Signal::new(None);
#[prop]
let id: String = String::from("combobox");
#[prop]
let label: String = String::new();
#[prop]
let list_label: String = String::new();

#[dispatcher(comboboxopen, optionchoose, comboboxdismiss)]
let events;
let field: Signal<Option<NodeId>> = Signal::new(None);
let list_id = format!("{id}-listbox");
let option_id = {
    let id = id.clone();
    move |index: usize| format!("{id}-option-{index}")
};
let active_id = {
    let option_id = option_id.clone();
    move || active.get().map(&option_id).unwrap_or_default()
};

// Each option row's node, under the row's key (its index).
let option_nodes: Signal<std::collections::BTreeMap<usize, NodeId>> = Signal::new(std::collections::BTreeMap::new());

let keys = move |event: &mut Event| {
    let count = options.with_untracked(Vec::len);
    let key = event.key().to_owned();
    match key.as_str() {
        "ArrowDown" | "ArrowUp" => {
            event.prevent_default();
            let down = key == "ArrowDown";
            if count == 0 {
                return;
            }
            if expanded.get_untracked() {
                let next = match (active.get_untracked(), down) {
                    (Some(at), true) => (at + 1) % count,
                    (Some(at), false) => (at + count - 1) % count,
                    (None, true) => 0,
                    (None, false) => count - 1,
                };
                active.set(Some(next));
            } else if field.get_untracked().is_some_and(|node| events.emit(node, Event::new("comboboxopen"))) {
                expanded.set(true);
                active.set(Some(if down { 0 } else { count - 1 }));
            }
        }
        "Home" | "End" => active.set(None),
        "Enter" => {
            event.prevent_default();
            if let Some(at) = active.get_untracked() {
                let chosen = option_nodes
                    .with_untracked(|nodes| nodes.get(&at).copied())
                    .is_some_and(|node| events.emit(node, Event::new("optionchoose")));
                if !chosen {
                    return;
                }
                if let Some(text) = options.with_untracked(|all| all.get(at).cloned()) {
                    value.set(text);
                }
            }
            expanded.set(false);
            active.set(None);
        }
        "Escape" => {
            event.prevent_default();
            if !field.get_untracked().is_some_and(|node| events.emit(node, Event::new("comboboxdismiss"))) {
                return;
            }
            if expanded.get_untracked() {
                expanded.set(false);
                active.set(None);
            } else {
                value.set(String::new());
            }
        }
        _ => {}
    }
};
</script>

<input id={id} type="text" role="combobox" aria-autocomplete="list" aria-label={label} aria-expanded={expanded.get()} aria-controls={list_id} aria-activedescendant={active_id()} bind:value={value} bind:this={field} onkeydown={keys} />{if expanded.get()}<div id={list_id} role="listbox" aria-label={list_label} class="combobox-list">{for row in options.get().into_iter().enumerate().collect::<Vec<_>>() key=row.0}<div id={option_id(row.get().0)} role="option" aria-selected={active.get() == Some(row.get().0)} bind:this={option_nodes}>{row.get().1}</div>{/for}</div>{/if}
