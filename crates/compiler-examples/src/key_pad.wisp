<script>
let x: Signal<i32> = Signal::new(0);
#[prop]
let heard: Signal<Vec<String>> = Signal::new(Vec::new());
let on_button_key = move |event: &mut Event| {
    heard.update(|h| h.push(format!("button {}", event.key())));
};
let on_page_key = move |event: &mut Event| {
    heard.update(|h| h.push(format!("window {}", event.key())));
    match event.key() {
        "ArrowLeft" => x.update(|v| *v -= 1),
        "ArrowRight" => x.update(|v| *v += 1),
        _ => {}
    }
};
</script>

<button onkeydown={on_button_key}>{x}</button>
<vapors:window onkeydown={on_page_key} />
