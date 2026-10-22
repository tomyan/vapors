//! A hidden `<input type="file">` appended to the page, clicked, read on
//! `change` and removed on `change` or `cancel`. Two closures are leaked
//! per pick, which is the cost of callbacks the browser may never call
//! (an old browser fires neither event for a cancelled picker).

use wasm_bindgen::prelude::*;
use web_sys::{File, HtmlInputElement};
use winit::event_loop::EventLoopProxy;

use crate::tasks::send;
use crate::window::Message;

type JsResult<T> = Result<T, String>;

fn js(error: &JsValue) -> String {
    error.as_string().unwrap_or_else(|| format!("{error:?}"))
}

/// Append the input, hook its `change`, click it.
pub fn open(proxy: EventLoopProxy<Message>) -> JsResult<()> {
    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?;
    let body = document.body().ok_or("no <body>")?;
    let input: HtmlInputElement = document
        .create_element("input")
        .map_err(|e| js(&e))?
        .dyn_into()
        .map_err(|_| "created element is not an <input>".to_owned())?;
    input.set_type("file");
    input.set_accept("image/*");
    input.set_hidden(true);
    body.append_child(&input).map_err(|e| js(&e))?;
    let on_change = Closure::once_into_js(move |event: web_sys::Event| chosen(&event, &proxy));
    input.set_onchange(Some(on_change.unchecked_ref()));
    let on_cancel = Closure::once_into_js(move |event: web_sys::Event| cancelled(&event));
    input.set_oncancel(Some(on_cancel.unchecked_ref()));
    input.click();
    Ok(())
}

fn input_of(event: &web_sys::Event) -> Option<HtmlInputElement> {
    event
        .target()
        .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
}

/// The `cancel` handler: the input has nothing more to say.
fn cancelled(event: &web_sys::Event) {
    if let Some(input) = input_of(event) {
        input.remove();
    }
}

/// The `change` handler: remove the input, take the first file, read it.
/// A change with no files is a cancel in browsers that report it that
/// way, so it sends nothing, as a cancel must.
fn chosen(event: &web_sys::Event, proxy: &EventLoopProxy<Message>) {
    let Some(input) = input_of(event) else {
        return;
    };
    input.remove();
    let Some(file) = input.files().and_then(|files| files.get(0)) else {
        return;
    };
    let proxy = proxy.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match read(&file).await {
            Ok(bytes) => send(
                &proxy,
                Message::FileChosen {
                    name: file.name(),
                    bytes,
                },
            ),
            Err(message) => {
                crate::report::report_error(&format!("could not read {}: {message}", file.name()));
            }
        }
    });
}

async fn read(file: &File) -> JsResult<Vec<u8>> {
    let buffer = wasm_bindgen_futures::JsFuture::from(file.array_buffer())
        .await
        .map_err(|e| js(&e))?;
    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}
