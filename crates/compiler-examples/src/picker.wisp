<script>
#[dispatcher(pick)]
let events;
let picked: Signal<bool> = Signal::new(false);
let refused: Signal<Option<String>> = Signal::new(None);
let choose = move |event: &mut Event| {
    let Some(target) = event.target() else {
        refused.set(Some("a click with no target".to_owned()));
        return;
    };
    match events.dispatch(target, Event::new("pick")) {
        Ok(went) => picked.set(went),
        Err(why) => refused.set(Some(why.to_string())),
    }
};
</script>

<button onclick={choose}>Pick</button>
