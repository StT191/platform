
#[cfg(not(target_family="wasm"))]
mod native;

#[cfg(not(target_family="wasm"))]
pub use native::*;


#[cfg(all(target_family="wasm", web_sys_unstable_apis))]
mod web;

#[cfg(all(target_family="wasm", web_sys_unstable_apis))]
pub use web::*;



// facade type with implicit error-handling

use anyhow::{Result as Res, Error, anyhow};

pub struct LocalStorageFacade {
    inner: Res<LocalStorage>,
    error_handler: Box<dyn Fn(&Error)>,
}

impl LocalStorageFacade {

    pub fn inner(&self) -> &Res<LocalStorage> { &self.inner }

    pub fn from_inner(inner: Res<LocalStorage>, error_handler: impl Fn(&Error) + 'static) -> Self {
        Self { inner, error_handler: Box::new(error_handler) }
    }

    pub fn from_storage(inner: LocalStorage, error_handler: impl Fn(&Error) + 'static) -> Self {
        Self::from_inner(Ok(inner), error_handler)
    }

    pub fn new(qualifier: &str, organization: &str, application: &str, error_handler: impl Fn(&Error) + 'static) -> Self {
        let inner = LocalStorage::new(qualifier, organization, application).inspect_err(&error_handler);
        Self::from_inner(inner, error_handler)
    }

    pub fn empty() -> Self {
        Self::from_inner(Err(anyhow!("Empty LocalStorageFacade")), |_err| {})
    }

    pub fn set(&self, key: &str, value: &str) {
        if let Ok(storage) = &self.inner {
            storage.set(key, value).unwrap_or_else(|err| (self.error_handler)(&err));
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.inner.as_ref().ok().map(|storage|
            storage.get(key).inspect_err(&self.error_handler).ok().flatten()
        ).flatten()
    }

    pub fn remove(&self, key: &str) {
        if let Ok(storage) = &self.inner {
            storage.remove(key).unwrap_or_else(|err| (self.error_handler)(&err));
        }
    }

    pub fn clear_all(&self) {
        if let Ok(storage) = &self.inner {
            storage.clear_all().unwrap_or_else(|err| (self.error_handler)(&err));
        }
    }
}