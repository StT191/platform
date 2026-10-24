
use std::sync::Arc;
use winit::window::Window;
use super::*;

// app ctx

#[cfg(any(feature = "timeout", feature = "async_timeout", feature = "frame_pacing"))]
use crate::time::*;

#[cfg(feature = "futures")]
pub struct AppFutureSpawner<U: EventLike = !> { futures: AppFutureRuntime<U> }

#[cfg(target_os = "android")]
use crate::android_util::*;


pub struct AppCtx<U: EventLike = !> {

  // pub
  #[cfg(feature = "auto_wake_lock")] pub auto_wake_lock: bool,
  pub exit: bool,

  // private
  #[cfg(any(feature = "timeout", feature = "async_timeout", feature = "frame_pacing"))]
  pub(super) timer: AppTimer,

  #[cfg(feature = "futures")] futures: AppFutureSpawner<U>,

  #[cfg(feature = "frame_pacing")] pub(super) frame_time: Instant,
  #[cfg(feature = "frame_pacing")] pub(super) schedule_frame: bool,
  #[cfg(feature = "frame_pacing")] pub(super) frame_timeout: Option<Instant>,

  #[cfg(feature = "frame_pacing")] manual_frame_duration: Option<Duration>,
  #[cfg(feature = "frame_pacing")] monitor_frame_duration: Option<Duration>,
  #[cfg(feature = "frame_pacing")] pub(super) frame_duration: Duration,

  pub(super) event_dispatcher: AppCtxEventDispatcher<U>,
  pub(super) window: Arc<Window>,

  #[cfg(target_os = "android")] pub(super) android_app: AndroidApp,
  #[cfg(target_os = "android")] pub(super) fullscreen: bool,
}

impl<U: EventLike> AppCtx<U> {

  pub fn window(&self) -> &Window { &self.window }

  pub fn window_clone(&self) -> Arc<Window> { self.window.clone() }

  #[cfg(target_os = "android")]
  pub fn android_app(&self) -> &AndroidApp { &self.android_app }

  pub fn new(
    futures: AppFutureRuntime<U>, timer: AppTimer, event_dispatcher: AppCtxEventDispatcher<U>, window: Window,
    #[cfg(target_os = "android")] android_app: AndroidApp,
  ) -> Self {

    #[cfg(not(feature = "futures"))] let _ = futures;

    #[cfg(not(any(feature = "timeout", feature = "async_timeout", feature = "frame_pacing")))]
    let _ = timer;

    #[allow(unused_mut)]
    let mut app_ctx = Self {

      #[cfg(feature = "futures")] futures: AppFutureSpawner::new(futures),

      #[cfg(any(feature = "timeout", feature = "async_timeout", feature = "frame_pacing"))]
      timer,

      #[cfg(feature = "frame_pacing")] frame_time: Instant::now(),
      #[cfg(feature = "frame_pacing")] schedule_frame: false,
      #[cfg(feature = "frame_pacing")] frame_timeout: None,
      #[cfg(feature = "frame_pacing")] manual_frame_duration: None,
      #[cfg(feature = "frame_pacing")] monitor_frame_duration: None,
      #[cfg(feature = "frame_pacing")] frame_duration: STD_FRAME_DURATION,
      #[cfg(feature = "auto_wake_lock")] auto_wake_lock: false,
      exit: false,

      event_dispatcher,

      // window_id: window.id(),
      window: Arc::new(window),

      #[cfg(target_os = "android")] android_app,
      #[cfg(target_os = "android")] fullscreen: false,
    };

    #[cfg(feature = "frame_pacing")]
    app_ctx.fetch_monitor_frame_duration();

    app_ctx
  }
}


// fullscreen handling
impl<U: EventLike> AppCtx<U> {

  pub fn is_fullscreen(&self) -> bool {
    #[cfg(not(target_os = "android"))] { self.window.fullscreen().is_some() }
    #[cfg(target_os = "android")] { self.fullscreen }
  }

  pub fn enable_fullscreen(&mut self) {
    #[cfg(not(target_os = "android"))] {
      self.window.set_fullscreen(Some(DEFAULT_FULLSCREEN));
    }
    #[cfg(target_os = "android")] {
      enable_fullscreen(&self.android_app);
      self.fullscreen = true;
    }
  }

  pub fn disable_fullscreen(&mut self) {
    #[cfg(not(target_os = "android"))] {
      self.window.set_fullscreen(None);
    }
    #[cfg(target_os = "android")] {
      disable_fullscreen(&self.android_app);
      self.fullscreen = false;
    }
  }

  pub fn toggle_fullscreen(&mut self) -> bool {
    if self.is_fullscreen() { self.disable_fullscreen(); false }
    else { self.enable_fullscreen(); true }
  }
}


// event dispatcher
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


// timeout feature
#[cfg(feature = "timeout")]
impl<U: EventLike> AppCtx<U> {

  pub fn get_timeout(&self, id: u64) -> Option<Instant> {
    self.timer.borrow().get_timeout(&AppTimeoutId::User(id))
  }

  pub fn set_timeout(&mut self, id: u64, instant: Instant) -> TimeoutResult {
    self.timer.borrow_mut().set_timeout(AppTimeoutId::User(id), instant)
  }

  pub fn set_timeout_earlier(&mut self, id: u64, instant: Instant) -> TimeoutResult {
    self.timer.borrow_mut().set_timeout_earlier(AppTimeoutId::User(id), instant)
  }

  pub fn set_timeout_wait(&mut self, id: u64, duraion: Duration) -> TimeoutResult {
    self.timer.borrow_mut().set_timeout_wait(AppTimeoutId::User(id), duraion)
  }

  pub fn set_timeout_wait_earlier(&mut self, id: u64, duraion: Duration) -> TimeoutResult {
    self.timer.borrow_mut().set_timeout_wait_earlier(AppTimeoutId::User(id), duraion)
  }

  pub fn cancel_timeout(&mut self, id: u64, if_later_than: Option<Instant>) -> TimeoutResult {
    self.timer.borrow_mut().cancel_timeout(&AppTimeoutId::User(id), if_later_than)
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


// futures feature
#[cfg(feature = "futures")]
impl<U: EventLike> Clone for AppFutureSpawner<U> {
  fn clone(&self) -> Self { Self { futures: self.futures.clone() } }
}

#[cfg(feature = "futures")]
impl<U: EventLike> AppFutureSpawner<U> {

  pub fn new(futures: AppFutureRuntime<U>) -> Self { Self {futures} }

  pub fn spawn(&self, future: AppFuture) -> AppFutureId {
    self.futures.spawn(future)
  }

  pub fn cancel(&self, id: &AppFutureId) {
    self.futures.cancel(id)
  }
}

#[cfg(feature = "futures")]
impl<U: EventLike> AppCtx<U> {

  pub fn future_spawner(&self) -> AppFutureSpawner<U> {
    self.futures.clone()
  }

  pub fn spawn_future(&self, future: AppFuture) -> AppFutureId {
    self.futures.spawn(future)
  }

  pub fn cancel_future(&self, id: &AppFutureId) {
    self.futures.cancel(id)
  }
}


// frame-pacing feature
#[cfg(feature = "frame_pacing")]
impl<U: EventLike> AppCtx<U> {

  pub fn frame_time(&self) -> Instant { self.frame_time }

  pub fn frame_timeout(&self) -> Option<Instant> { self.frame_timeout }

  pub fn schedule_frame(&mut self, instant: Instant) {

    let set_instant = match self.frame_timeout {
      Some(other) if other <= instant => None,
      // if no previous or earlier than previous
      _ => Some(instant),
    };

    if set_instant.is_some() {
      self.frame_timeout = set_instant;
      self.schedule_frame = true;
    }
  }

  pub fn schedule_frame_timeout(&mut self, duraion: Duration) {
    if let Some(instant) = Instant::now().checked_add(duraion) {
      self.schedule_frame(instant);
    }
  }

  pub fn request_frame(&mut self) {
    self.schedule_frame(self.frame_time + self.frame_duration);
  }

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