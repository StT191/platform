
use std::future::Future;


pub fn spawn_local<F: Future<Output = ()> + 'static>(future: F) {
    #[cfg(not(target_family="wasm"))] pollster::block_on(future);
    #[cfg(target_family="wasm")] wasm_bindgen_futures::spawn_local(future);
}


pub trait SpawnFutureLocal: Future<Output = ()> + 'static {
    fn spawn_local(self);
}

impl<F: Future<Output = ()> + 'static> SpawnFutureLocal for F {
    fn spawn_local(self) { spawn_local(self) }
}
