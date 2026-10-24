
use anyhow::{Result as Res};
use nosleep::{NoSleep, NoSleepType};

pub struct WakeLock {
    nosleep: NoSleep,
    active: bool,
}

impl WakeLock {

    pub fn new() -> Res<Self> {
        Ok(Self { nosleep: NoSleep::new()?, active: false })
    }

    pub fn is_active(&self) -> bool { self.active }

    pub fn request(&mut self) -> Res<()> {
        if !self.active {
            self.nosleep.start(NoSleepType::PreventUserIdleDisplaySleep)?;
            self.active = true;
            log::trace!("WakeLock requested");
        }
        Ok(())
    }

    pub fn release(&mut self) -> Res<()> {
        if self.active {
            self.nosleep.stop()?;
            self.active = false;
            log::trace!("WakeLock released");
        }
        Ok(())
    }
}