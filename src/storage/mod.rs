
#[cfg(not(target_family="wasm"))]
mod native;

#[cfg(not(target_family="wasm"))]
pub use native::*;


#[cfg(all(target_family="wasm", web_sys_unstable_apis))]
mod web;

#[cfg(all(target_family="wasm", web_sys_unstable_apis))]
pub use web::*;