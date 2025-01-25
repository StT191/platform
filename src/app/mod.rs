
use std::future::Future;
use std::pin::Pin;
use winit::{window::WindowId, event::{*, Event as WinitEvent}, event_loop::*};
use crate::*;

// mods
mod ctx;
mod state;
mod mount;

pub use ctx::*;
use state::*;
pub use mount::*;


#[cfg(feature = "frame_pacing")]
pub const STD_FRAME_DURATION: time::Duration = time::Duration::from_nanos(10u64.pow(9)/60);


#[cfg(any(feature = "app_timer", all(feature = "frame_pacing", not(target_family = "wasm"))))]
mod timer;


#[cfg(all(feature = "web_clipboard", target_family="wasm", web_sys_unstable_apis))]
mod web_clipboard;

#[cfg(all(feature = "web_clipboard", target_family="wasm", web_sys_unstable_apis))]
pub use web_clipboard::*;


// types

#[derive(Debug, Clone, PartialEq)]
pub enum AppEventExt {
  AppInit { window_id: WindowId },
  #[cfg(feature = "app_waker")] Wake { window_id: WindowId, wake_id: u128 },
  #[cfg(all(feature = "web_clipboard", target_family="wasm"))] ClipboardFetch { window_id: WindowId },
  #[cfg(all(feature = "web_clipboard", target_family="wasm"))] ClipboardPaste { window_id: WindowId },
}

pub type AppEventLoop = EventLoop<AppEventExt>;
pub type AppEventLoopProxy = EventLoopProxy<AppEventExt>;
pub type AppEvent = WinitEvent<AppEventExt>;
pub type AppEventLoopClosed = EventLoopClosed<AppEventExt>;


#[derive(Debug, Clone)]
pub enum Event {
  Resumed,
  Suspended,
  WindowEvent(WindowEvent),
  #[cfg(feature = "device_events")] DeviceEvent { device_id: DeviceId, event: DeviceEvent },
  #[cfg(feature = "app_timer")] Timeout { instant: time::Instant, id: u128 },
  #[cfg(feature = "app_waker")] Wake(u128),
  #[cfg(all(feature = "web_clipboard", target_family="wasm"))] ClipboardFetch,
  #[cfg(all(feature = "web_clipboard", target_family="wasm"))] ClipboardPaste,
}


pub trait AppHandler: Sized + 'static {

  type InitData;

  fn init(app_ctx: &mut AppCtx, init_data: Self::InitData) -> impl Future<Output=Self>;

  fn event(&mut self, app_ctx: &mut AppCtx, event: &Event);
}



// wrapper for fn-handler types

pub struct AppClosure<H> where
  H: FnMut(&mut AppCtx, &Event) + Sized + 'static
{
  handler: H,
}


type BoxFuture<'a, T> = Pin<Box<dyn Future<Output=T> + 'a>>;


impl <H> AppHandler for AppClosure<H> where
  H: FnMut(&mut AppCtx, &Event) + Sized + 'static
{

  type InitData = Box<dyn FnOnce(&mut AppCtx) -> BoxFuture<'_, H>>;

  async fn init(app_ctx: &mut AppCtx, init_fn: Self::InitData) -> Self {
    Self { handler: init_fn(app_ctx).await }
  }

  fn event(&mut self, app_ctx: &mut AppCtx, event: &Event) {
    (self.handler)(app_ctx, event)
  }
}


// helper macros to initialize and start apps

#[macro_export]
macro_rules! app_closure {
  ($init_fn:expr) => {
    ::std::boxed::Box::new(|app_ctx| {
      ::std::boxed::Box::pin($init_fn(app_ctx))
    })
  }
}


#[macro_export]
macro_rules! main_app {
  ($log_level:expr, $window_attributes:expr, $init_data:expr $(,)?) => {
    fn main() {
      $crate::init($log_level);
      $crate::AppMount::start($window_attributes, $init_data);
    }
  };
  ($log_level:expr, $window_attributes:expr, $app_type:ty, $init_data:expr $(,)?) => {
    fn main() {
      $crate::init($log_level);
      $crate::AppMount::<$app_type>::start($window_attributes, $init_data);
    }
  };
}


#[macro_export]
macro_rules! main_app_closure {
  ($log_level:expr, $window_attributes:expr, $init_fn:expr $(,)?) => {
    $crate::main_app!(
      $log_level, $window_attributes,
      $crate::AppClosure::<_>, $crate::app_closure!($init_fn),
    );
  };
}
