<script>
#[prop] let id: String = crate::unique_id("hold");
#[prop] let label: String = String::from("Hold");
#[prop] let pressed: Signal<bool> = Signal::new(false);
#[prop] let enabled: Signal<bool> = Signal::new(true);
#[prop] let reset: Signal<u64> = Signal::new(0);
// aria-disabled exposes availability; input guards enforce it without removing focusability.
// Each pointer remains independent, including two fingers on the same button.
let pointers: Signal<std::collections::BTreeSet<u32>> = Signal::new(std::collections::BTreeSet::new());
let keys: Signal<std::collections::BTreeSet<String>> = Signal::new(std::collections::BTreeSet::new());
Effect::new(move || { pressed.set(!pointers.get().is_empty() || !keys.get().is_empty()); });
let reset_seen = std::rc::Rc::new(std::cell::Cell::new(reset.get_untracked()));
Effect::new(move || {
    let epoch = reset.get();
    let active = enabled.get();
    if reset_seen.replace(epoch) != epoch || !active {
        pointers.set(std::collections::BTreeSet::new());
        keys.set(std::collections::BTreeSet::new());
    }
});
vapors_signals::on_cleanup(move || { crate::clear_hold(pressed); });
let down = move |event: &mut Event| -> Result<(), String> {
    if !enabled.get_untracked() { return Ok(()); }
    let pointer = event.pointer().filter(|p| p.pressed)
        .ok_or_else(|| String::from("HoldButton pointerdown requires a pressed pointer"))?;
    event.prevent_default();
    event.set_pointer_capture();
    pointers.update(|held| { held.insert(pointer.id); });
    Ok(())
};
let release = move |event: &mut Event| -> Result<(), String> {
    let pointer = event.pointer()
        .ok_or_else(|| String::from("HoldButton pointer release requires a pointer"))?;
    pointers.update(|held| { held.remove(&pointer.id); });
    Ok(())
};
let key_down = move |event: &mut Event| {
    if enabled.get_untracked() && matches!(event.key(), " " | "Enter") {
        event.prevent_default();
        keys.update(|held| { held.insert(event.key().to_owned()); });
    }
};
let key_up = move |event: &mut Event| {
    if matches!(event.key(), " " | "Enter") {
        event.prevent_default();
        keys.update(|held| { held.remove(event.key()); });
    }
};
let blur = move || { keys.set(std::collections::BTreeSet::new()); };
</script>

<style>
button { display: block; pointer-events: auto; width: var(--hold-width, 68px); height: var(--hold-height, 60px); padding: 6px; border: 1px solid #526a69; border-radius: 5px; font-size: 16px; font-weight: 700; background-color: var(--hold-background, #20383d); color: var(--hold-color, #f3f1e8); }
button[data-held="true"] { border-color: #f2b26f; }
button:focus-visible { outline: 2px solid #f2b26f; outline-offset: 2px; }
</style>

<button id={id} data-hold={id} data-held={pressed.get()} aria-disabled={!enabled.get()}
    onpointerdown={down} onpointerup={release} onpointercancel={release}
    onlostpointercapture={release} onkeydown={key_down} onkeyup={key_up} onblur={blur}>{label}</button>
