#![feature(never_type, associated_type_defaults)]

// re-exports
pub use winit;
pub use web_time as time;
pub use rel_path::*;

#[cfg(not(target_family="wasm"))]
pub use pollster;

#[cfg(all(feature="directories", not(target_family="wasm")))]
pub use robius_directories as directories;

// mods
pub mod anyhow; // als re-exports the anyhow crate

mod logger;
pub use logger::*;

mod init;
pub use init::*;

mod future;
pub use future::*;

mod conditional_execution;
pub use conditional_execution::*;

mod runtime;
pub use runtime::*;

mod norm_path;
pub use norm_path::*;

mod app;
pub use app::*;

#[cfg(feature="touches")]
pub mod touches;

pub mod timer;

#[cfg(feature="storage")]
pub mod storage;

#[cfg(feature="rng")]
pub mod rng;

#[cfg(any(feature="rapidhash", feature="rng"))]
pub mod rapidhash {
    pub use ::rapidhash::{*, fast::*};
}

#[cfg(all(feature="icon_loader", target_os="linux"))]
pub mod icon_loader;

#[cfg(feature="wake_lock")]
pub mod wake_lock;


#[macro_export]
macro_rules! shrink_capacity {
    ($coll:expr) => {
        if $coll.len() <= $coll.capacity() / 4 {
          $coll.shrink_to($coll.capacity() / 2);
        }
    };
    ($coll:expr, $min:expr) => {
        if $coll.capacity() > const {2*$min} && $coll.len() <= $coll.capacity() / 4 {
          $coll.shrink_to($coll.capacity() / 2);
        }
    };
}