
use std::{rc::Rc, cell::RefCell};
use anyhow::{Result as Res, Context, bail};
use wasm_bindgen::{JsValue};
use wasm_bindgen_futures::{JsFuture};
use web_sys::{WakeLock as WebWakeLock, WakeLockType, WakeLockSentinel};

pub struct WakeLock {
    locker: Box<WebWakeLock>,
    lock: Rc<RefCell<Option<WakeLockSentinel>>>,
}

impl WakeLock {

    pub fn new() -> Res<Self> {
        let window = web_sys::window().context("couldn't get web_sys::Window")?;

        if !JsValue::from("wakeLock").js_in(&window.navigator()) {
            bail!("navigator.wakeLock is not supported");
        }

        Ok(Self {
            locker: window.navigator().wake_lock().into(),
            lock: RefCell::new(None).into()
        })
    }

    pub fn is_active(&self) -> bool { self.lock.borrow().is_some() }

    pub fn request(&mut self) -> Res<()> {
        let _res = self.release();

        let lock = Rc::clone(&self.lock);
        let promise = self.locker.request(WakeLockType::Screen);

        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(res) = JsFuture::from(promise).await {
                lock.replace(Some(WakeLockSentinel::from(res)));
            }
        });

        Ok(())
    }

    pub fn release(&mut self) -> Res<()> {

        if let Some(lock) = self.lock.borrow().as_ref() {
            let _res = lock.release();
        }

        self.lock.replace(None);

        Ok(())
    }
}