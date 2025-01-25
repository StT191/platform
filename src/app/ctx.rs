
use std::sync::Arc;
use winit::window::Window;
use super::*;

// app futures feature

#[cfg(feature = "futures")]
#[derive(Clone)]
pub struct AppFutureSpawner<U: EventLike = !> { futures: AppFutureRuntime<U> }

#[cfg(feature = "futures")]
impl<U: EventLike> AppFutureSpawner<U> {

  pub fn new(futures: AppFutureRuntime<U>) -> Self { Self {futures} }

  pub fn spawn(&mut self, future: AppFuture) -> AppFutureId {
    self.futures.spawn(future)
  }

  pub fn cancel(&mut self, id: &AppFutureId) {
    self.futures.cancel(id)
  }

}


#[cfg(any(feature = "timeout", feature = "async_timeout", feature = "frame_pacing"))]
use crate::time::*;


pub struct AppCtx<U: EventLike = !> {

  #[cfg(feature = "futures")] pub futures: AppFutureSpawner<U>,

  #[cfg(any(feature = "timeout", feature = "async_timeout", feature = "frame_pacing"))]
  pub(super) timer: AppTimer,

  #[cfg(feature = "frame_pacing")] pub(super) frame_request: bool,
  #[cfg(feature = "frame_pacing")] pub(super) frame_time: Instant,

  #[cfg(feature = "frame_pacing")] manual_frame_duration: Option<Duration>,
  #[cfg(feature = "frame_pacing")] monitor_frame_duration: Option<Duration>,
  #[cfg(feature = "frame_pacing")] pub(super) frame_duration: Duration,

  #[cfg(feature = "auto_wake_lock")] pub auto_wake_lock: bool,

  pub exit: bool,

  pub(super) event_dispatcher: AppCtxEventDispatcher<U>,

  #[allow(unused)] window_id: WindowId,

  pub(super) window: Arc<Window>,
}

impl<U: EventLike> AppCtx<U> {

  pub fn window(&self) -> &Window { &self.window }

  pub fn window_clone(&self) -> Arc<Window> { self.window.clone() }

  pub fn new(futures: AppFutureRuntime<U>, timer: AppTimer, event_dispatcher: AppCtxEventDispatcher<U>, window: Window) -> Self {

    #[cfg(not(feature = "futures"))] let _ = futures;

    #[cfg(not(any(feature = "timeout", feature = "async_timeout", feature = "frame_pacing")))]
    let _ = timer;

    #[allow(unused_mut)]
    let mut app_ctx = Self {

      #[cfg(feature = "futures")] futures: AppFutureSpawner::new(futures),

      #[cfg(any(feature = "timeout", feature = "async_timeout", feature = "frame_pacing"))]
      timer,

      #[cfg(feature = "frame_pacing")] frame_request: false,
      #[cfg(feature = "frame_pacing")] manual_frame_duration: None,
      #[cfg(feature = "frame_pacing")] monitor_frame_duration: None,
      #[cfg(feature = "frame_pacing")] frame_duration: STD_FRAME_DURATION,
      #[cfg(feature = "frame_pacing")] frame_time: Instant::now(),
      #[cfg(feature = "auto_wake_lock")] auto_wake_lock: false,
      exit: false,

      event_dispatcher,

      window_id: window.id(),
      window: Arc::new(window),
    };

    #[cfg(feature = "frame_pacing")]
    app_ctx.fetch_monitor_frame_duration();

    app_ctx
  }
}


// timeout feature

#[cfg(feature = "timeout")]
use std::ops::ControlFlow;

#[cfg(feature = "timeout")]
impl<U: EventLike> AppCtx<U> {

  pub fn get_timeout(&self, id: u64) -> Option<Instant> {
    self.timer.borrow().get_timeout(&AppTimeoutId::User(id))
  }

  pub fn set_timeout(&mut self, id: u64, instant: Instant) -> Option<Instant> {
    self.timer.borrow_mut().set_timeout(AppTimeoutId::User(id), instant)
  }

  pub fn set_timeout_earlier(&mut self, id: u64, instant: Instant) -> ControlFlow<Instant, Option<Instant>> {
    self.timer.borrow_mut().set_timeout_earlier(AppTimeoutId::User(id), instant)
  }

  pub fn set_timeout_wait(&mut self, id: u64, duraion: Duration) -> Option<Instant> {
    self.timer.borrow_mut().set_timeout_wait(AppTimeoutId::User(id), duraion)
  }

  pub fn set_timeout_wait_earlier(&mut self, id: u64, duraion: Duration) -> ControlFlow<Instant, Option<Instant>> {
    self.timer.borrow_mut().set_timeout_wait_earlier(AppTimeoutId::User(id), duraion)
  }

  pub fn cancel_timeout(&mut self, id: u64, if_later: Option<Instant>) -> ControlFlow<Option<Instant>, Instant> {
    self.timer.borrow_mut().cancel_timeout(&AppTimeoutId::User(id), if_later)
  }
}


// async-timeout feature

#[cfg(feature = "async_timeout")]
impl<U: EventLike> AppCtx<U> {

  pub fn async_timeout(&self, instant: Instant) -> AsyncTimeout<AppTimeoutId> {
    AsyncTimeout::new(self.timer.clone(), instant)
  }

  pub fn async_wait(&self, duraion: Duration) -> Option<AsyncTimeout<AppTimeoutId>> {
    Instant::now().checked_add(duraion).map(|instant| self.async_timeout(instant))
  }

  pub fn async_timer(&self) -> AsyncTimer<AppTimeoutId> {
    AsyncTimer::new(self.timer.clone())
  }

}


// frame-pacing feature

#[cfg(feature = "frame_pacing")]
impl<U: EventLike> AppCtx<U> {

  pub fn request_frame(&mut self) { self.frame_request = true; }

  pub fn frame_time(&self) -> Instant { self.frame_time }

  #[cfg(not(target_family = "wasm"))]
  pub fn sync_frame_time(&mut self, instant: Instant) {
    self.frame_time = instant.min(Instant::now() + self.frame_duration);
  }

  fn update_frame_duration(&mut self) {
    self.frame_duration = self.manual_frame_duration.or(self.monitor_frame_duration).unwrap_or(STD_FRAME_DURATION);
  }

  pub fn set_frame_duration(&mut self, manual_duration: Option<Duration>) {
    self.manual_frame_duration = manual_duration;
    self.update_frame_duration();
  }

  pub fn fetch_monitor_frame_duration(&mut self) -> Option<Duration> {
    self.monitor_frame_duration = self.window.frame_duration();
    self.update_frame_duration();
    self.monitor_frame_duration
  }

  pub fn manual_frame_duration(&mut self) -> Option<Duration> { self.manual_frame_duration }

  pub fn monitor_frame_duration(&mut self) -> Option<Duration> { self.monitor_frame_duration }

  pub fn frame_duration(&mut self) -> Duration { self.frame_duration }

}


// app-waker feature

#[derive(Debug, Clone)]
pub struct AppEventDispatcher<U: EventLike> {
  event_dispatcher: AppCtxEventDispatcher<U>,
}

impl<U: EventLike> AppEventDispatcher<U> {
  pub fn dispatch(&self, event: U) {
    self.event_dispatcher.dispatch(AppEventExt::UserEvent(event));
  }
}

impl<U: EventLike> AppCtx<U> {

  pub fn dispatch_event(&self, event: U) {
    self.event_dispatcher.dispatch(AppEventExt::UserEvent(event));
  }

  pub fn event_dispatcher(&self) -> AppEventDispatcher<U> {
    AppEventDispatcher { event_dispatcher: self.event_dispatcher.clone() }
  }
}