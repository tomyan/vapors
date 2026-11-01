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
// - When the app's options change, an active option no longer at its
//   place is cleared, as APG's example resets it while filtering.
// A dispatch that is refused, or a choice that cannot be resolved, is
// the key listener's `Err`, which goes to the app's error report.
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
let id: String = crate::unique_id("combobox");
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

// The field's node; only an unmounted combobox has none.
let field_node = move || {
    field
        .get_untracked()
        .ok_or_else(|| "the combobox's field is not mounted".to_owned())
};
// Fires `name` at `node`: whether no listener cancelled it.
let fire = move |node: NodeId, name: &str| events.dispatch(node, Event::new(name)).map_err(|refused| refused.to_string());

// An active option the new options no longer hold at its place goes.
let shown_options = std::rc::Rc::new(std::cell::RefCell::new(options.get_untracked()));
Effect::new(move || {
    let now = options.get();
    let was = shown_options.replace(now.clone());
    if active.get_untracked().is_some_and(|at| was.get(at) != now.get(at)) {
        active.set(None);
    }
});

// `field_node` holds only a signal, so it is Copy; `fire` holds the
// dispatcher, so each handler takes its own.
let arrow = {
let fire = fire.clone();
move |down: bool| -> Result<(), String> {
        let count = options.with_untracked(Vec::len);
        if count == 0 {
            return Ok(());
        }
        if expanded.get_untracked() {
            let next = match (active.get_untracked(), down) {
                (Some(at), true) => (at + 1) % count,
                (Some(at), false) => (at + count - 1) % count,
                (None, true) => 0,
                (None, false) => count - 1,
            };
            active.set(Some(next));
        } else if fire(field_node()?, "comboboxopen")? {
            expanded.set(true);
            active.set(Some(if down { 0 } else { count - 1 }));
        }
        Ok(())
}
};

let enter = {
let fire = fire.clone();
move || -> Result<(), String> {
        if let Some(at) = active.get_untracked() {
            let node = option_nodes
                .with_untracked(|nodes| nodes.get(&at).copied())
                .ok_or_else(|| format!("the active option {at} has no element"))?;
            if !fire(node, "optionchoose")? {
                return Ok(());
            }
            let text = options
                .with_untracked(|all| all.get(at).cloned())
                .ok_or_else(|| format!("the active option {at} is not among the options"))?;
            value.set(text);
        }
        expanded.set(false);
        active.set(None);
        Ok(())
}
};

let escape = move || -> Result<(), String> {
    if !fire(field_node()?, "comboboxdismiss")? {
        return Ok(());
    }
    if expanded.get_untracked() {
        expanded.set(false);
        active.set(None);
    } else {
        value.set(String::new());
    }
    Ok(())
};

let keys = move |event: &mut Event| -> Result<(), String> {
    match event.key() {
        "ArrowDown" | "ArrowUp" => {
            event.prevent_default();
            let down = event.key() == "ArrowDown";
            arrow(down)
        }
        "Home" | "End" => {
            active.set(None);
            Ok(())
        }
        "Enter" => {
            event.prevent_default();
            enter()
        }
        "Escape" => {
            event.prevent_default();
            escape()
        }
        _ => Ok(()),
    }
};
</script>

<input id={id} type="text" role="combobox" aria-autocomplete="list" aria-label={label} aria-expanded={expanded.get()} aria-controls={list_id} aria-activedescendant={active_id()} bind:value={value} bind:this={field} onkeydown={keys} />{if expanded.get()}<div id={list_id} role="listbox" aria-label={list_label} class="combobox-list">{for row in options.get().into_iter().enumerate().collect::<Vec<_>>() key=row.0}<div id={option_id(row.get().0)} role="option" aria-selected={active.get() == Some(row.get().0)} bind:this={option_nodes}>{row.get().1}</div>{/for}</div>{/if}
