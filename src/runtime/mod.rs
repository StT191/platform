
use std::{fmt::Debug, marker::Sync, hash::Hash};
use winit::{window::WindowId, event::{*, Event}, event_loop::*, application::ApplicationHandler};
use crate::*;

// mods

mod futures;
pub use futures::{Futures as RuntimeFutures, FutureRuntime};
use RuntimeFutures as Futures;


#[cfg(any(feature="timeout", feature="async_timeout", all(feature="frame_pacing", not(target_family="wasm"))))]
mod timer;

#[cfg(any(feature="timeout", feature="async_timeout", all(feature="frame_pacing", not(target_family="wasm"))))]
pub use self::{
  timer::{RuntimeTimer, Timer},
  futures::{AsyncTimeoutId, AsyncTimeout, AsyncTimer},
};

#[cfg(not(any(feature="timeout", feature="async_timeout", all(feature="frame_pacing", not(target_family="wasm")))))]
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
      #[cfg(any(feature="timeout", feature="async_timeout", all(feature="frame_pacing", not(target_family="wasm"))))]
      timer: std::rc::Rc::new(Timer::new().into()),

      #[cfg(not(any(feature="timeout", feature="async_timeout", all(feature="frame_pacing", not(target_family="wasm")))))]
      timer: std::marker::PhantomData,

      futures: FutureRuntime::new(event_loop_proxy.clone()),
      event_dispatcher: EventDispatcher {event_loop_proxy},
    }
  }
}


#[derive(Debug)]
pub enum RuntimeEvent<F: Futures, U: EventLike, T: IdLike> {
  Resumed,
  Suspended,
  Exit,
  FutureReady {id: F::Id, output: <F::Future as Future>::Output},
  UserEvent(U),
  WindowEvent {window_id: WindowId, event: WindowEvent},
  #[cfg(feature = "device_events")] DeviceEvent {device_id: DeviceId, event: DeviceEvent},

  #[cfg(any(feature="timeout", feature="async_timeout", all(feature="frame_pacing", not(target_family="wasm"))))]
  Timeout {id: T, instant: time::Instant},

  #[cfg(not(any(feature="timeout", feature="async_timeout", all(feature="frame_pacing", not(target_family="wasm")))))]
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

  pub fn event(&mut self, event_loop: &ActiveEventLoop, event: Event<RuntimeEventExt<R::FutureId, R::UserEvent>>) {

    let &mut Self { ref mut ctx, ref mut runtime, .. } = self;

    let mut check_timer = true;

    match event {

      #[cfg(any(feature="timeout", feature="async_timeout", all(feature="frame_pacing", not(target_family="wasm"))))]
      Event::NewEvents(StartCause::ResumeTimeReached {..}) => {
        if let Some(timer::Timeout {id, instant}) = {ctx.timer.borrow_mut().pop_timeout()} {
          runtime.event(event_loop, ctx, RuntimeEvent::Timeout {id, instant});
        }
        else { check_timer = false }
      },

      Event::WindowEvent {window_id: id, event} => runtime.event(event_loop, ctx, RuntimeEvent::WindowEvent {window_id: id, event}),

      Event::UserEvent(RuntimeEventExt::WakeFuture(id)) => {
        if let Some(output) = ctx.futures.poll(&id) {
          runtime.event(event_loop, ctx, RuntimeEvent::FutureReady {id, output})
        }
        else { check_timer = false }
      },

      Event::UserEvent(RuntimeEventExt::UserEvent(event)) => {
        runtime.event(event_loop, ctx, RuntimeEvent::UserEvent(event))
      },

      #[cfg(feature = "device_events")]
      Event::DeviceEvent {device_id, event} => runtime.event(event_loop, ctx, RuntimeEvent::DeviceEvent{device_id, event}),

      Event::Resumed => runtime.event(event_loop, ctx, RuntimeEvent::Resumed),

      Event::Suspended => runtime.event(event_loop, ctx, RuntimeEvent::Suspended),

      Event::LoopExiting => {
        runtime.event(event_loop, ctx, RuntimeEvent::Exit);
        check_timer = false;
      },

      _ => { check_timer = false },

    };

    #[cfg(any(feature="timeout", feature="async_timeout", all(feature="frame_pacing", not(target_family="wasm"))))]
    if check_timer {
      if let Some(set_instant) = ctx.timer.borrow_mut().take_set_instant() {
        match set_instant {
          Some(instant) => event_loop.set_wait_until(instant),
          None => event_loop.set_wait(),
        }
      }
    }

    #[cfg(not(any(feature="timeout", feature="async_timeout", all(feature="frame_pacing", not(target_family="wasm")))))]
    let _ = check_timer;

  }

}


impl<R: Runtime> ApplicationHandler<RuntimeEventExt<R::FutureId, R::UserEvent>> for RuntimeMount<R> {

  fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
    self.event(event_loop, Event::NewEvents(cause));
  }

  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    self.event(event_loop, Event::Resumed);
  }

  fn suspended(&mut self, event_loop: &ActiveEventLoop) {
    self.event(event_loop, Event::Suspended);
  }

  fn user_event(&mut self, event_loop: &ActiveEventLoop, event: RuntimeEventExt<R::FutureId, R::UserEvent>) {
    self.event(event_loop, Event::UserEvent(event));
  }

  fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
    self.event(event_loop, Event::WindowEvent { window_id, event });
  }

  fn exiting(&mut self, event_loop: &ActiveEventLoop) {
    self.event(event_loop, Event::LoopExiting);
  }

  #[cfg(feature = "device_events")]
  fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
    self.event(event_loop, Event::DeviceEvent { device_id, event });
  }

}