
use winit::window::WindowId;
use std::{rc::Rc, cell::RefCell};
use crate::{*};
use web_sys::{Clipboard as DomClipboard, ClipboardEvent};
use js_sys::Function;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen::{JsValue, closure::Closure};


// helper

#[derive(Debug)]
struct ClipboardHandle { clipboard: DomClipboard, readable: bool, writeable: bool }

impl ClipboardHandle {
  fn new() -> Option<Self> {
    (|| {
      let clipboard = web_sys::window()
        .map(|win| win.navigator().clipboard())
        .ok_or("navigator.clipboard is not available")?
      ;

      let readable = JsValue::from("readText").js_in(&clipboard);
      let writeable = JsValue::from("writeText").js_in(&clipboard);

      Ok(Self { clipboard, readable, writeable })
    })
    ().map_err(|m: &str| log::warn!("{m:?}")).ok()
  }
}


struct PasteListener { listener: Function }

impl PasteListener {

  fn new(clipboard_content: Rc<RefCell<Option<String>>>, event_proxy: Option<(AppEventLoopProxy, WindowId)>) -> Self {

    let closure: Box<dyn Fn(ClipboardEvent)> = if let Some((event_loop_proxy, window_id)) = event_proxy {
      Box::new(move |evt| {
        if let Some(transfer) = evt.clipboard_data() {
          clipboard_content.replace(transfer.get_data("text").map_err(|m| log::error!("{m:?}")).ok());
        }
        event_loop_proxy.send_event(AppEventExt::ClipboardPaste {window_id}).unwrap_or_else(|m| log::error!("{m:?}"));
      })
    }
    else {
      Box::new(move |evt| {
        if let Some(transfer) = evt.clipboard_data() {
          clipboard_content.replace(transfer.get_data("text").map_err(|m| log::error!("{m:?}")).ok());
        }
      })
    };

    Self { listener: Closure::wrap(closure).into_js_value().into() }
  }

  fn attach(&self) -> Result<(), &'static str> {
    let document = web_sys::window().and_then(|win| win.document()).ok_or("couldn't get window.document")?;

    document.add_event_listener_with_callback("paste", &self.listener)
      .or(Err("couldn't attach PasteListener to document"))?;

    Ok(())
  }

  fn detach(&self) {
    (|| {
      let document = web_sys::window().and_then(|win| win.document()).ok_or("couldn't get window.document")?;

      document.remove_event_listener_with_callback("paste", &self.listener)
        .or(Err("couldn't detach PasteListener to document"))?;

      Ok(())
    })
    ().unwrap_or_else(|m: &str| log::error!("{m:?}"));
  }

  fn attached(clipboard_content: Rc<RefCell<Option<String>>>, event_proxy: Option<(AppEventLoopProxy, WindowId)>) -> Option<Self> {
    let listener = Self::new(clipboard_content, event_proxy);
    listener.attach().map(|()| listener).map_err(|m| log::error!("{m:?}")).ok()
  }
}


// main clipboard

pub struct WebClipboard {
  content: Rc<RefCell<Option<String>>>,
  handle: Option<ClipboardHandle>,
  paste_listener: Option<PasteListener>,
  event_proxy: Option<(AppEventLoopProxy, WindowId)>,
}

impl WebClipboard {

  pub fn connect(app_ctx: &AppCtx, attach_listener: bool) -> Self {

    let window_id = app_ctx.window().id();
    let event_loop_proxy = app_ctx.event_loop_proxy.clone();

    let content = RefCell::new(None).into();

    let paste_listener = match attach_listener {
      true => PasteListener::attached(Rc::clone(&content), Some((event_loop_proxy.clone(), window_id))),
      false => None,
    };

    Self {
      content, handle: ClipboardHandle::new(), paste_listener,
      event_proxy: Some((event_loop_proxy, window_id)),
    }
  }

  pub fn unconnected(attach_listener: bool) -> Self {

    let content = RefCell::new(None).into();

    let paste_listener = match attach_listener {
      true => PasteListener::attached(Rc::clone(&content), None),
      false => None,
    };

    Self {
      content, handle: ClipboardHandle::new(), paste_listener,
      event_proxy: None,
    }
  }

  pub fn fetch(&self) { // fetches content from system clipboard asynchronously
    if let Some(ClipboardHandle {readable: true, clipboard, ..}) = &self.handle {

      let content = Rc::clone(&self.content);
      let promise = clipboard.read_text();

      if let Some((event_loop_proxy, window_id)) = &self.event_proxy {
        let event_loop_proxy = event_loop_proxy.clone();
        let window_id = *window_id;

        wasm_bindgen_futures::spawn_local(async move {
          content.replace(
            JsFuture::from(promise).await
            .map_err(|m| log::error!("{m:?}")).ok()
            .and_then(|res| res.as_string())
          );
          event_loop_proxy.send_event(AppEventExt::ClipboardFetch {window_id}).unwrap_or_else(|m| log::error!("{m:?}"));
        });
      }
      else {
        wasm_bindgen_futures::spawn_local(async move {
          content.replace(
            JsFuture::from(promise).await
            .map_err(|m| log::error!("{m:?}")).ok()
            .and_then(|res| res.as_string())
          );
        });
      }
    }
  }

  pub fn read(&self) -> Option<String> {
    self.content.borrow().as_ref().cloned()
  }

  pub fn write(&mut self, text: String) {
    if let Some(ClipboardHandle {writeable: true, clipboard, ..}) = &self.handle {
      let _promise = clipboard.write_text(&text);
    }
    self.content.replace(Some(text));
  }


  // introspective methods

  pub fn is_connected(&self) -> bool {
    self.event_proxy.is_some()
  }

  pub fn is_listening(&self) -> bool {
    self.paste_listener.is_some()
  }

  pub fn is_readable(&self) -> bool {
    matches!(self.handle, Some(ClipboardHandle {readable: true, ..}))
  }

  pub fn is_writeable(&self) -> bool {
    matches!(self.handle, Some(ClipboardHandle {writeable: true, ..}))
  }

}


impl Drop for WebClipboard {
  fn drop(&mut self) {
    if let Some(listener) = &self.paste_listener {
      listener.detach()
    }
  }
}

use std::fmt;

impl fmt::Debug for WebClipboard {
  fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt.debug_struct("WebClipboard")
    .field("connected", &self.is_connected())
    .field("listening", &self.is_listening())
    .field("readable", &self.is_readable())
    .field("writeable", &self.is_writeable())
    .finish()
  }
}