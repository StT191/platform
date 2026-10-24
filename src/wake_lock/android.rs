
use crate::{anyhow::Result as Res, android_util::*};


pub struct WakeLock {
    app: AndroidApp,
    active: bool,
}

impl WakeLock {

    pub fn new(app: &AndroidApp) -> Res<Self> {
        Ok(Self { app: app.clone(), active: false })
    }

    pub fn is_active(&self) -> bool { self.active }

    pub fn request(&mut self) -> Res<()> {
        enable_wake_lock(&self.app);
        self.active = true;
        Ok(())
    }

    pub fn release(&mut self) -> Res<()> {
        disable_wake_lock(&self.app);
        self.active = false;
        Ok(())
    }
}