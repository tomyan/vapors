//! A hidden `<input type="file">` appended to the page, clicked, and read
//! on `change`. One closure is leaked per pick, which is the cost of a
//! callback the browser may never call (a cancelled picker fires nothing).

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
    input.click();
    Ok(())
}

/// The `change` handler: take the first file, read it, remove the input.
fn chosen(event: &web_sys::Event, proxy: &EventLoopProxy<Message>) {
    let Some(input) = event
        .target()
        .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
    else {
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
