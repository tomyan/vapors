<script>
#[dispatcher(pick)]
let events;
let picked: Signal<bool> = Signal::new(false);
let choose = move |event: &mut Event| -> Result<(), vapors_dom::DispatchRefused> {
    let Some(target) = event.target() else {
        return Ok(());
    };
    picked.set(events.dispatch(target, Event::new("pick"))?);
    Ok(())
};
</script>

<button onclick={choose}>Pick</button>
