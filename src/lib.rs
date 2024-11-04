
// re-exports
pub use winit;
pub use web_time as time;
pub use log::{self, Level as LogLevel};
pub use anyhow;

#[cfg(not(target_family="wasm"))]
pub use pollster;

#[cfg(all(feature="directories", not(target_family="wasm")))]
pub use robius_directories as directories;

// mods
mod platform_winit;
pub use platform_winit::*;

mod future;
pub use future::*;

mod conditional_execution;
pub use conditional_execution::*;

mod app;
pub use app::*;

#[cfg(feature="touches")]
pub mod touches;

mod log_helper;

pub mod timer;

#[cfg(feature="storage")]
pub mod storage;

#[cfg(feature="rng")]
pub mod rng;

#[cfg(all(feature="icon_loader", target_os="linux"))]
pub mod icon_loader;

#[cfg(feature="wake_lock")]
pub mod wake_lock;