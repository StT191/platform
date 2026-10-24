
// on native
#[cfg(not(any(target_family = "wasm", target_os = "android")))]
mod native;

#[cfg(not(any(target_family = "wasm", target_os = "android")))]
pub use native::*;

// on the web
#[cfg(all(target_family="wasm", web_sys_unstable_apis))]
mod web;

#[cfg(all(target_family="wasm", web_sys_unstable_apis))]
pub use web::*;

// on android
#[cfg(target_os="android")]
mod android;

#[cfg(target_os="android")]
pub use android::*;