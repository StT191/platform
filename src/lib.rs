
// re-exports
pub use winit;
pub use web_time as time;
pub use log::{self, Level as LogLevel};
pub use anyhow;

#[cfg(not(target_family="wasm"))]
pub use pollster;

// mods
mod platform;
pub use platform::*;

mod log_helper;

mod app;
pub use app::*;

pub mod timer;

// rng
#[cfg(feature = "rng")]
pub mod rng;

// icon loader
#[cfg(feature = "icon_loader")]
#[cfg(target_os = "linux")]
pub mod icon_loader;

// wake_lock
#[cfg(feature = "wake_lock")]
pub mod wake_lock;