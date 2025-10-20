
use std::{fmt::Debug, marker::Sync, hash::Hash};
use winit::{window::WindowId, event::*, event_loop::*, application::ApplicationHandler};
use crate::*;

// mods

mod futures;
pub use futures::{Futures as RuntimeFutures, FutureRuntime};
use RuntimeFutures as Futures;


#[cfg(any(feature="timeout", feature="async_timeout", feature="frame_pacing"))]
mod timer;

#[cfg(any(feature="timeout", feature="async_timeout", feature="frame_pacing"))]
pub use self::{
  timer::{RuntimeTimer, Timer, TimeoutResult},
  futures::{AsyncTimeoutId, AsyncTimeout, AsyncTimer},
};

#[cfg(not(any(feature="timeout", feature="async_timeout", feature="frame_pacing")))]
pub type RuntimeTimer<T> = std::marker::PhantomData<T>;


// types

pub trait EventLike: Debug + Send + 'static {}
impl<T: Debug + Send + 'static> EventLike for T {}

pub trait IdLike: EventLike + Sync + PartialEq {}
impl<T: EventLike + Sync + PartialEq> IdLike for T {}


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum RuntimeEventExt<I: IdLike, U: EventLike> {
  WakeFuture(I),
  UserEvent(U),
}

pub type RuntimeEventLoop<I, U> = EventLoop<RuntimeEventExt<I, U>>;
pub type RuntimeEventLoopProxy<I, U> = EventLoopProxy<RuntimeEventExt<I, U>>;


#[derive(Debug)]
pub struct EventDispatcher<I: IdLike, U: EventLike> {
  event_loop_proxy: RuntimeEventLoopProxy<I, U>
}

impl<I: IdLike, U: EventLike> Clone for EventDispatcher<I, U> {
  fn clone(&self) -> Self { Self { event_loop_proxy: self.event_loop_proxy.clone() } }
}

impl<I: IdLike, U: EventLike> EventDispatcher<I, U> {
  pub fn dispatch(&self, event: U) {
    self.event_loop_proxy
      .send_event(RuntimeEventExt::UserEvent(event))
      .unwrap_or_else(|m| log::error!("{m:?}"))
    ;
  }
}


#[derive(Debug)]
pub struct RuntimeCtx<F: Futures, U: EventLike, T: IdLike> {
  pub timer: RuntimeTimer<T>,
  pub futures: FutureRuntime<F, U>,
  pub event_dispatcher: EventDispatcher<F::Id, U>,
}

impl<F: Futures, U: EventLike, T: IdLike> RuntimeCtx<F, U, T> {

  pub(super) fn new(event_loop_proxy: RuntimeEventLoopProxy<F::Id, U>) -> Self {
    Self {
      #[cfg(any(feature="timeout", feature="async_timeout", feature="frame_pacing"))]
      timer: std::rc::Rc::new(Timer::new().into()),

      #[cfg(not(any(feature="timeout", feature="async_timeout", feature="frame_pacing")))]
      timer: std::marker::PhantomData,

      futures: FutureRuntime::new(event_loop_proxy.clone()),
      event_dispatcher: EventDispatcher {event_loop_proxy},
    }
  }
}


#[derive(derive_more::Debug)]
pub enum RuntimeEvent<F: Futures, U: EventLike, T: IdLike> {
  Resumed,
  Suspended,
  Exit,
  FutureReady {id: F::Id, output: <F::Future as Future>::Output},
  UserEvent(U),
  WindowEvent {window_id: WindowId, event: WindowEvent},
  #[cfg(feature = "device_events")] DeviceEvent {device_id: DeviceId, event: DeviceEvent},

  #[cfg(any(feature="timeout", feature="async_timeout", feature="frame_pacing"))]
  Timeout {id: T, instant: time::Instant},

  #[cfg(not(any(feature="timeout", feature="async_timeout", feature="frame_pacing")))]
  Timeout((!, std::marker::PhantomData<T>)),
}


pub trait Runtime: 'static {

  type FutureId: IdLike;
  type Futures: Futures<Id=Self::FutureId>;

  type UserEvent: EventLike;
  type TimeoutId: IdLike;

  fn event(
    &mut self,
    event_loop: &ActiveEventLoop,
    ctx: &mut RuntimeCtx<Self::Futures, Self::UserEvent, Self::TimeoutId>,
    event: RuntimeEvent<Self::Futures, Self::UserEvent, Self::TimeoutId>,
  );

  fn run(self, event_loop: RuntimeEventLoop<Self::FutureId, Self::UserEvent>) where Self: Sized {
    RuntimeMount::new(event_loop.create_proxy(), self).run(event_loop);
  }

  fn start(self) where Self: Sized { self.run(event_loop()) }
}


pub struct RuntimeMount<R: Runtime> {
  pub(super) ctx: RuntimeCtx<R::Futures, R::UserEvent, R::TimeoutId>,
  pub(super) runtime: R,
}

impl<R: Runtime> RuntimeMount<R> {

  pub fn new(event_loop_proxy: RuntimeEventLoopProxy<R::FutureId, R::UserEvent>, runtime: R) -> Self {
    Self { ctx: RuntimeCtx::new(event_loop_proxy), runtime }
  }

  pub fn run(self, event_loop: RuntimeEventLoop<R::FutureId, R::UserEvent>) {

    #[cfg(not(target_family="wasm"))] {
      let mut runtime = self;
      event_loop.run_app(&mut runtime).unwrap();
    }

    #[cfg(target_family="wasm")] {
      use winit::platform::web::EventLoopExtWebSys;
      event_loop.spawn_app(self);
    }
  }

  pub fn start(runtime: R) {
    let event_loop = event_loop();
    Self::new(event_loop.create_proxy(), runtime).run(event_loop);
  }

  pub fn event(&mut self, event_loop: &ActiveEventLoop, event: RuntimeEvent<R::Futures, R::UserEvent, R::TimeoutId>) {

    self.runtime.event(event_loop, &mut self.ctx, event);

    #[cfg(any(feature="timeout", feature="async_timeout", feature="frame_pacing"))]
    if let Some(set_instant) = self.ctx.timer.borrow_mut().take_set_instant() {
      match set_instant {
        Some(instant) => event_loop.set_wait_until(instant),
        None => event_loop.set_wait(),
      }
    }
  }
}


impl<R: Runtime> ApplicationHandler<RuntimeEventExt<R::FutureId, R::UserEvent>> for RuntimeMount<R> {

  #[cfg(any(feature="timeout", feature="async_timeout", feature="frame_pacing"))]
  fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
    if matches!(cause, StartCause::ResumeTimeReached {..}) {
      if let Some(timer::Timeout {id, instant}) = { self.ctx.timer.borrow_mut().pop_timeout() } {
        self.event(event_loop, RuntimeEvent::Timeout {id, instant});
      }
    }
  }

  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    self.event(event_loop, RuntimeEvent::Resumed);
  }

  fn suspended(&mut self, event_loop: &ActiveEventLoop) {
    self.event(event_loop, RuntimeEvent::Suspended);
  }

  fn user_event(&mut self, event_loop: &ActiveEventLoop, event: RuntimeEventExt<R::FutureId, R::UserEvent>) {
    match event {
      RuntimeEventExt::WakeFuture(id) => {
        if let Some(output) = self.ctx.futures.poll(&id) {
          self.event(event_loop, RuntimeEvent::FutureReady {id, output});
        }
      },
      RuntimeEventExt::UserEvent(event) => {
        self.event(event_loop, RuntimeEvent::UserEvent(event));
      },
    }
  }

  fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
    self.event(event_loop, RuntimeEvent::WindowEvent { window_id, event });
  }

  fn exiting(&mut self, event_loop: &ActiveEventLoop) {
    self.event(event_loop, RuntimeEvent::Exit);
  }

  #[cfg(feature = "device_events")]
  fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
    self.event(event_loop, RuntimeEvent::DeviceEvent { device_id, event });
  }

}