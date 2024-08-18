
// platform types

use winit::event_loop::{
    EventLoop as WinitEventLoop,
    EventLoopProxy as WinitEventLoopProxy,
    EventLoopWindowTarget as WinitEventLoopWindowTarget,
};
use winit::event::Event as WinitEventType;
use winit::window::WindowId;


#[derive(Debug, Clone, PartialEq)]
pub enum PlatformEventExt {
    AppInit { window_id: WindowId },
    #[cfg(target_family="wasm")] ClipboardPaste { window_id: WindowId },
    #[cfg(target_family="wasm")] ClipboardFetch { window_id: WindowId },
}

pub type PlatformEventLoop = WinitEventLoop<PlatformEventExt>;
pub type PlatformEventLoopProxy = WinitEventLoopProxy<PlatformEventExt>;
pub type PlatformEventLoopWindowTarget = WinitEventLoopWindowTarget<PlatformEventExt>;
pub type PlatformEvent = WinitEventType<PlatformEventExt>;


// submods

mod future;
pub use future::*;

mod conditional_execution;
pub use conditional_execution::*;

mod init;
pub use init::*;


// extension

use winit::event_loop::{EventLoopWindowTarget, ControlFlow};
use crate::time::Instant;

pub trait ControlFlowExtension {
    fn set_poll(&self);
    fn set_wait(&self);
    fn set_wait_until(&self, instant: Instant);
    fn set_earlier(&self, instant: Instant);
}

impl<T> ControlFlowExtension for EventLoopWindowTarget<T> {

    fn set_poll(&self) { self.set_control_flow(ControlFlow::Poll); }
    fn set_wait(&self) { self.set_control_flow(ControlFlow::Wait); }
    fn set_wait_until(&self, instant: Instant) { self.set_control_flow(ControlFlow::WaitUntil(instant)); }

    fn set_earlier(&self, instant: Instant) {
        match self.control_flow() {
            ControlFlow::Poll => {},
            ControlFlow::Wait => self.set_wait_until(instant),
            ControlFlow::WaitUntil(other) => self.set_wait_until(instant.min(other)),
        }
    }
}