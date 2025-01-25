
use std::sync::Arc;
use winit::window::Window;
use super::*;


#[cfg(any(feature = "app_timer", all(feature = "frame_pacing", not(target_family = "wasm"))))]
use super::timer::*;

#[cfg(any(feature = "app_timer", feature = "frame_pacing"))]
use crate::time::*;

#[cfg(feature = "app_timer")]
use std::ops::ControlFlow;


#[derive(Debug)]
pub struct AppCtx {

  #[cfg(any(feature = "app_timer", all(feature = "frame_pacing", not(target_family = "wasm"))))]
  pub(super) timer: AppTimer,

  #[cfg(feature = "frame_pacing")] pub(super) frame_request: bool,
  #[cfg(feature = "frame_pacing")] pub(super) frame_time: Instant,

  #[cfg(feature = "frame_pacing")] manual_frame_duration: Option<Duration>,
  #[cfg(feature = "frame_pacing")] monitor_frame_duration: Option<Duration>,
  #[cfg(feature = "frame_pacing")] pub(super) frame_duration: Duration,

  #[cfg(feature = "auto_wake_lock")] pub auto_wake_lock: bool,

  pub exit: bool,

  #[allow(unused)] pub(super) event_loop_proxy: AppEventLoopProxy,
  pub(crate) window: Arc<Window>,
}


impl AppCtx {

  pub fn window(&self) -> &Window { &self.window }

  pub fn window_clone(&self) -> Arc<Window> { self.window.clone() }

  #[cfg(feature = "app_timer")]
  pub fn get_timeout(&mut self, id: u128) -> Option<Instant> {
    self.timer.get_timeout_pos(TimerId::User(id)).map(|(_i, timer)| timer.instant)
  }

  #[cfg(feature = "app_timer")]
  pub fn set_timeout(&mut self, id: u128, instant: Instant) -> Option<Instant> {
    match self.timer.set_timeout(Timeout {instant, id: TimerId::User(id)}, false) {
      ControlFlow::Continue(res) => res,
      ControlFlow::Break(_instant) => unreachable!(),
    }
  }

  #[cfg(feature = "app_timer")]
  pub fn set_timeout_earlier(&mut self, id: u128, instant: Instant) -> ControlFlow<Instant, Option<Instant>> {
    self.timer.set_timeout(Timeout {instant, id: TimerId::User(id)}, true)
  }

  #[cfg(feature = "app_timer")]
  pub fn set_timeout_wait(&mut self, id: u128, duraion: Duration) -> Option<Instant> {
    if let Some(instant) = Instant::now().checked_add(duraion) {
      match self.timer.set_timeout(Timeout {instant, id: TimerId::User(id)}, false) {
        ControlFlow::Continue(res) => res,
        ControlFlow::Break(_) => unreachable!(),
      }
    } else { match self.timer.cancel_timeout(TimerId::User(id), None) {
      ControlFlow::Break(None) => None,
      ControlFlow::Continue(instant) => Some(instant),
      ControlFlow::Break(Some(_)) => unreachable!(),
    }}
  }

  #[cfg(feature = "app_timer")]
  pub fn set_timeout_wait_earlier(&mut self, id: u128, duraion: Duration) -> ControlFlow<Instant, Option<Instant>> {
    if let Some(instant) = Instant::now().checked_add(duraion) {
      self.timer.set_timeout(Timeout {instant, id: TimerId::User(id)}, true)
    }
    else if let Some(instant) = self.get_timeout(id) {
      // MAX is always later than any instant
      ControlFlow::Break(instant)
    }
    else { match self.timer.cancel_timeout(TimerId::User(id), None) {
      ControlFlow::Break(None) => ControlFlow::Continue(None),
      ControlFlow::Continue(instant) => ControlFlow::Continue(Some(instant)),
      ControlFlow::Break(Some(_)) => unreachable!(),
    }}
  }

  #[cfg(feature = "app_timer")]
  pub fn cancel_timeout(&mut self, id: u128, if_later: Option<Instant>) -> ControlFlow<Option<Instant>, Instant> {
    self.timer.cancel_timeout(TimerId::User(id), if_later)
  }


  #[cfg(feature = "frame_pacing")]
  pub fn request_frame(&mut self) { self.frame_request = true; }

  #[cfg(feature = "frame_pacing")]
  pub fn frame_time(&self) -> Instant { self.frame_time }

  #[cfg(all(feature = "frame_pacing", not(target_family = "wasm")))]
  pub fn sync_frame_time(&mut self, instant: Instant) {
    self.frame_time = instant.min(Instant::now() + self.frame_duration);
  }

  #[cfg(feature = "frame_pacing")]
  fn update_frame_duration(&mut self) {
    self.frame_duration = self.manual_frame_duration.or(self.monitor_frame_duration).unwrap_or(STD_FRAME_DURATION);
  }

  #[cfg(feature = "frame_pacing")]
  pub fn set_frame_duration(&mut self, manual_duration: Option<Duration>) {
    self.manual_frame_duration = manual_duration;
    self.update_frame_duration();
  }

  #[cfg(feature = "frame_pacing")]
  pub fn fetch_monitor_frame_duration(&mut self) -> Option<Duration> {
    self.monitor_frame_duration = self.window.frame_duration();
    self.update_frame_duration();
    self.monitor_frame_duration
  }

  #[cfg(feature = "frame_pacing")]
  pub fn manual_frame_duration(&mut self) -> Option<Duration> { self.manual_frame_duration }

  #[cfg(feature = "frame_pacing")]
  pub fn monitor_frame_duration(&mut self) -> Option<Duration> { self.monitor_frame_duration }

  #[cfg(feature = "frame_pacing")]
  pub fn frame_duration(&mut self) -> Duration { self.frame_duration }


  #[cfg(feature = "app_waker")]
  pub fn wake(&self, wake_id: u128) {
    self.event_loop_proxy
      .send_event(AppEventExt::Wake {window_id: self.window.id(), wake_id})
      .unwrap_or_else(|m| log::error!("{m:?}"))
    ;
  }

  pub(super) fn new(event_loop_proxy: AppEventLoopProxy, window: Window) -> Self {

    #[allow(unused_mut)]
    let mut app_ctx = Self {
      #[cfg(any(feature = "app_timer", all(feature = "frame_pacing", not(target_family = "wasm"))))]
      timer: AppTimer::default(),

      #[cfg(feature = "frame_pacing")] frame_request: false,
      #[cfg(feature = "frame_pacing")] manual_frame_duration: None,
      #[cfg(feature = "frame_pacing")] monitor_frame_duration: None,
      #[cfg(feature = "frame_pacing")] frame_duration: STD_FRAME_DURATION,

      #[cfg(feature = "frame_pacing")] frame_time: Instant::now(),
      #[cfg(feature = "auto_wake_lock")] auto_wake_lock: false,
      exit: false,
      event_loop_proxy,
      window: Arc::new(window),
    };

    #[cfg(feature = "frame_pacing")]
    app_ctx.fetch_monitor_frame_duration();

    app_ctx
  }

}