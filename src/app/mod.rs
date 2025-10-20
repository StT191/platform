
use std::{fmt::Debug, future::Future};
use winit::{window::{WindowId, WindowAttributes}, event::*};
use crate::*;

// mods
mod ctx;
mod state;
mod mount;
mod futures;

pub use ctx::*;
use state::*;
pub use mount::*;
pub use futures::*;


#[cfg(all(feature = "web_clipboard", target_family="wasm", web_sys_unstable_apis))]
mod web_clipboard;

#[cfg(all(feature = "web_clipboard", target_family="wasm", web_sys_unstable_apis))]
pub use web_clipboard::*;


// types

#[cfg(feature = "frame_pacing")]
pub const STD_FRAME_DURATION: time::Duration = time::Duration::from_nanos(10u64.pow(9)/60);


#[derive(Debug, Clone)]
pub enum AppEventExt<U: EventLike = !> {
  UserEvent(U),
  #[cfg(all(feature = "web_clipboard", target_family="wasm"))] ClipboardFetch(WindowId),
  #[cfg(all(feature = "web_clipboard", target_family="wasm"))] ClipboardPaste(WindowId),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppTimeoutId {
  #[cfg(feature = "timeout")] User(u64),
  #[cfg(feature = "frame_pacing")] FrameRequest(WindowId),
  #[cfg(feature = "async_timeout")] Async(AsyncTimeoutId),
}

pub type AppFuture = BoxFuture<'static, ()>;
pub type AppFutureId = <AppFutures as RuntimeFutures>::Id;
pub type AppTimer = RuntimeTimer<AppTimeoutId>;
pub type AppFutureRuntime<U=!> = FutureRuntime<AppFutures, AppEventExt<U>>;
pub type AppRuntimeCtx<U=!> = RuntimeCtx<AppFutures, AppEventExt<U>, AppTimeoutId>;
pub type AppEvent<U=!> = RuntimeEvent<AppFutures, AppEventExt<U>, AppTimeoutId>;
pub type AppCtxEventDispatcher<U=!> = EventDispatcher<AppFutureId, AppEventExt<U>>;


#[derive(Debug, Clone, PartialEq)]
pub enum Event<U: EventLike = !> {
  Resumed,
  Suspended,
  WindowEvent(WindowEvent),
  UserEvent(U),
  #[cfg(feature = "timeout")] Timeout {instant: time::Instant, id: u64},
  #[cfg(feature = "futures")] FutureReady(AppFutureId),
  #[cfg(all(feature = "web_clipboard", target_family="wasm"))] ClipboardFetch,
  #[cfg(all(feature = "web_clipboard", target_family="wasm"))] ClipboardPaste,
  #[cfg(feature = "device_events")] DeviceEvent {device_id: DeviceId, event: DeviceEvent},
}


// handler trait

pub trait AppHandler: Sized + 'static {

  type InitData = ();
  type UserEvent: EventLike = !;

  fn init(app_ctx: &mut AppCtx<Self::UserEvent>, init_data: Self::InitData) -> impl Future<Output=Self>;

  fn event(&mut self, app_ctx: &mut AppCtx<Self::UserEvent>, event: Event<Self::UserEvent>);

  fn mount(window_attributes: WindowAttributes, init_data: Self::InitData) -> AppMount<Self> {
    AppMount::new(window_attributes, init_data)
  }
}


// wrapper for fn-handler types

pub struct AppClosure<H, U = !> where
  H: FnMut(&mut AppCtx<U>, Event<U>) + Sized + 'static,
{
  handler: H,
  user_event_type: std::marker::PhantomData<U>,
}

impl <H, U: EventLike> AppHandler for AppClosure<H, U> where
  H: FnMut(&mut AppCtx<U>, Event<U>) + Sized + 'static
{

  type InitData = Box<dyn FnOnce(&mut AppCtx<U>) -> BoxFuture<'_, H>>;
  type UserEvent = U;

  async fn init(app_ctx: &mut AppCtx<U>, init_fn: Self::InitData) -> Self {
    Self { handler: init_fn(app_ctx).await, user_event_type: std::marker::PhantomData }
  }

  fn event(&mut self, app_ctx: &mut AppCtx<U>, event: Event<U>) {
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
  ($log_level:expr, $window_attributes:expr, $app_type:ty $(,)?) => {
    $crate::main_app!{$log_level, $window_attributes, $app_type, ()}
  };
  ($log_level:expr, $window_attributes:expr, $app_type:ty, $init_data:expr $(,)?) => {
    fn main() {
      $crate::init($log_level);
      $crate::AppMount::<$app_type>::new($window_attributes, $init_data).start();
    }
  };
}

#[macro_export]
macro_rules! main_app_closure {
  ($log_level:expr, $window_attributes:expr, $init_closure:expr $(,)?) => {
    $crate::main_app!{
      $log_level, $window_attributes,
      $crate::AppClosure::<_, _>, $crate::app_closure!($init_closure),
    }
  };
}
