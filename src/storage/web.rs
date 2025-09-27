
use anyhow::{Result as Res, Context, anyhow};
use web_sys::Storage;


#[derive(Debug, Clone)]
pub struct LocalStorage {
    storage: Storage,
}

impl LocalStorage {

    pub fn new(_qualifier: &str, _organization: &str, _application: &str) -> Res<Self> {

        let window = web_sys::window().context("couldn't get web_sys::Window")?;

        let storage = window.local_storage()
            .map_err(|_err| anyhow!("failed to acquire LocalStorage"))?
            .context("LocalStorage is not supported")?
        ;

        Ok(Self {storage})
    }

    pub fn set(&self, key: &str, value: &str) -> Res<()> {
        self.storage.set_item(key, value).map_err(|_err| anyhow!("failed writing to LocalStorage"))
    }

    pub fn get(&self, key: &str) -> Res<Option<String>> {
        self.storage.get_item(key).map_err(|_err| anyhow!("failed reading from LocalStorage"))
    }

    pub fn remove(&self, key: &str) -> Res<()> {
        self.storage.remove_item(key).map_err(|_err| anyhow!("failed to remove from LocalStorage"))
    }

    pub fn clear_all(&self) -> Res<()> {
        self.storage.clear().map_err(|_err| anyhow!("failed clearing LocalStorage"))
    }
}