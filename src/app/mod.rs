
use std::future::Future;
use std::pin::Pin;
use winit::event::{WindowEvent, DeviceId, DeviceEvent};
use crate::*;

// mods
mod ctx;
mod state;
mod mount;
pub use ctx::*;
use state::*;
pub use mount::*;


#[cfg(all(feature = "web_clipboard", target_family="wasm"))]
#[cfg(web_sys_unstable_apis)]
pub mod web_clipboard;


// exports
#[cfg(feature = "redraw_timer")]
use crate::time::Duration;

#[cfg(feature = "redraw_timer")] pub const STD_FRAME_TIME: Duration = Duration::from_nanos(10u64.pow(9)/60);
#[cfg(feature = "redraw_timer")] pub const STD_WAIT_TIME: Duration = Duration::from_millis(4);
#[cfg(feature = "redraw_timer")] pub const STD_FRAME_TIMEOUT: Duration = STD_FRAME_TIME.saturating_sub(STD_WAIT_TIME);


#[derive(Debug, Clone)]
pub enum AppEvent {
  Resumed,
  Suspended,
  WindowEvent(WindowEvent),
  DeviceEvent{device_id: DeviceId, event: DeviceEvent},
  #[cfg(all(feature = "web_clipboard", target_family="wasm"))] ClipboardFetch,
  #[cfg(all(feature = "web_clipboard", target_family="wasm"))] ClipboardPaste,
}


pub trait AppHandler: Sized + 'static {

  type InitData;

  fn init(app_ctx: &mut AppCtx, init_data: Self::InitData) -> impl Future<Output=Self>;

  fn event(&mut self, app_ctx: &mut AppCtx, event: &AppEvent);
}



// wrapper for fn-handler types

pub struct AppClosure<H> where
  H: FnMut(&mut AppCtx, &AppEvent) + Sized + 'static
{
  handler: H,
}


type BoxFuture<'a, T> = Pin<Box<dyn Future<Output=T> + 'a>>;


impl <H> AppHandler for AppClosure<H> where
  H: FnMut(&mut AppCtx, &AppEvent) + Sized + 'static
{

  type InitData = Box<dyn FnOnce(&mut AppCtx) -> BoxFuture<'_, H>>;

  async fn init(app_ctx: &mut AppCtx, init_fn: Self::InitData) -> Self {
    Self { handler: init_fn(app_ctx).await }
  }

  fn event(&mut self, app_ctx: &mut AppCtx, event: &AppEvent) {
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
  ($log_level:expr, $window_builder:expr, $init_data:expr $(,)?) => {
    fn main() {
      $crate::init($log_level);
      $crate::AppMount::start($window_builder, $init_data);
    }
  };
  ($log_level:expr, $window_builder:expr, $app_type:ty, $init_data:expr $(,)?) => {
    fn main() {
      $crate::init($log_level);
      $crate::AppMount::<$app_type>::start($window_builder, $init_data);
    }
  };
}


#[macro_export]
macro_rules! main_app_closure {
  ($log_level:expr, $window_builder:expr, $init_fn:expr $(,)?) => {
    $crate::main_app!(
      $log_level, $window_builder,
      $crate::AppClosure::<_>, $crate::app_closure!($init_fn),
    );
  };
}
