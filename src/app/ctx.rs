
use std::sync::Arc;
use winit::window::Window;
use crate::*;

#[cfg(feature = "redraw_timer")]
use crate::time::{Instant, Duration};


#[derive(Debug)]
pub struct AppCtx {
  #[cfg(feature = "redraw_timer")] pub redraw_time: Option<Instant>,
  #[cfg(feature = "auto_wake_lock")] pub auto_wake_lock: bool,
  pub exit: bool,
  event_loop_proxy: PlatformEventLoopProxy,
  window: Arc<Window>,
}


impl AppCtx {

  pub fn window(&self) -> &Window {
    &self.window
  }

  pub fn window_clone(&self) -> Arc<Window> {
    self.window.clone()
  }

  pub fn event_loop_proxy(&self) -> &PlatformEventLoopProxy {
    &self.event_loop_proxy
  }

  #[cfg(feature = "redraw_timer")]
  pub fn redraw_timeout(&mut self, timeout: Duration) -> bool {
    if let Some(time) = Instant::now().checked_add(timeout) {
      if let Some(earlier) = self.redraw_time {
        if earlier <= time { return false }
      }
      self.redraw_time = Some(time);
      true
    } else { false }
  }

  pub(super) fn new(event_loop_proxy: PlatformEventLoopProxy, window: Window) -> Self { Self {
    #[cfg(feature = "redraw_timer")] redraw_time: None,
    #[cfg(feature = "auto_wake_lock")] auto_wake_lock: false,
    exit: false,
    event_loop_proxy,
    window: Arc::new(window),
  }}

}