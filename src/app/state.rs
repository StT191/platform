
use winit::{window::WindowId, event::WindowEvent, event_loop::ActiveEventLoop};

use crate::{*, error::Res};

#[cfg(feature = "frame_timer")]
use crate::time::*;

#[cfg(feature = "frame_timer")]
use winit::event::StartCause;

#[cfg(feature = "auto_wake_lock")]
use crate::wake_lock::WakeLock;

use super::{AppEvent, AppCtx, AppHandler};


pub(super) struct AppState<App: AppHandler> {
  #[cfg(feature = "auto_wake_lock")] wake_lock: Option<WakeLock>,
  #[cfg(feature = "frame_timer")] animate: DetectChanges<bool>,
  #[cfg(feature = "frame_timer")] requested: DetectChanges<Option<Duration>>,
  #[cfg(feature = "frame_timer")] last: Instant,
  #[cfg(feature = "frame_timer")] next: Instant,
  window_id: WindowId,
  app_ctx: AppCtx,
  app: App,
}

impl<App: AppHandler> AppState<App> {

  pub(super) fn new(app_ctx: AppCtx, app: App) -> Self {
    Self {
      #[cfg(feature = "auto_wake_lock")] wake_lock: WakeLock::new().inspect_err(|err| log_warn!(err)).ok(),
      #[cfg(feature = "frame_timer")] animate: DetectChanges::new(!app_ctx.animate),
      #[cfg(feature = "frame_timer")] requested: DetectChanges::new(None),
      #[cfg(feature = "frame_timer")] last: Instant::now(),
      #[cfg(feature = "frame_timer")] next: Instant::now() + app_ctx.duration,
      window_id: app_ctx.window().id(),
      app_ctx, app,
    }
  }

  pub(super) fn event(&mut self, event: PlatformEvent, event_loop: &ActiveEventLoop) {

    let app_ctx = &mut self.app_ctx;

    match event {

      #[cfg(feature = "frame_timer")]
      PlatformEvent::NewEvents(StartCause::ResumeTimeReached {..}) => {
        app_ctx.window().request_redraw();
      },

      PlatformEvent::Resumed => {
        self.app.event(app_ctx, &AppEvent::Resumed);
        self.after_event(event_loop, None);
      },

      PlatformEvent::Suspended => {
        self.app.event(app_ctx, &AppEvent::Suspended);
        self.after_event(event_loop, None);
      },

      #[cfg(all(feature = "web_clipboard", target_family="wasm"))]
      PlatformEvent::UserEvent(user_event) => match user_event {
        PlatformEventExt::ClipboardFetch { window_id: id } if id == self.window_id => {
          self.app.event(app_ctx, &AppEvent::ClipboardFetch);
          self.after_event(event_loop, None);
        },
        PlatformEventExt::ClipboardPaste { window_id: id } if id == self.window_id  => {
          self.app.event(app_ctx, &AppEvent::ClipboardPaste);
          self.after_event(event_loop, None);
        },
        _ => {},
      },

      PlatformEvent::WindowEvent { window_id: id, event: window_event } if id == self.window_id => {

        #[cfg(feature = "auto_wake_lock")]
        let mut focus_change: Option<bool> = None;

        // before user handler
        match &window_event {

          #[cfg(feature = "frame_timer")]
          WindowEvent::RedrawRequested => {

            let now = Instant::now();

            app_ctx.request = None;
            self.requested.set_state(None);

            self.last = if self.next > now || (self.next + app_ctx.duration) <= now {
              now
            } else {
              self.next // avoid timer shifts
            };

            self.next = self.last + app_ctx.duration;

            if app_ctx.animate { event_loop.set_wait_until(self.next); }
            else { event_loop.set_wait(); }
          },

          WindowEvent::CloseRequested => {
            app_ctx.exit = true;
          },

          WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged {..} => {
            app_ctx.window().request_redraw();
          },

          #[cfg(feature = "auto_wake_lock")]
          WindowEvent::Focused(focus) => { focus_change = Some(*focus) },

          _ => {},
        }

        // exec event handler
        self.app.event(app_ctx, &AppEvent::WindowEvent(window_event));

        self.after_event(event_loop, {
          #[cfg(feature = "auto_wake_lock")] { focus_change }
          #[cfg(not(feature = "auto_wake_lock"))] { None }
        });
      },

      _ => {}

    }
  }

  fn after_event(&mut self, event_loop: &ActiveEventLoop, focus_change: Option<bool>) {

    let app_ctx = &mut self.app_ctx;

    if app_ctx.exit {
      event_loop.exit();
      return;
    }

    #[cfg(feature = "auto_wake_lock")]
    fn wake_lock(wake_lock: &mut Option<WakeLock>, action: impl FnOnce(&mut WakeLock) -> Res<()>) {
      if let Some(wake_lock) = wake_lock.as_mut() {
        action(wake_lock).unwrap_or_else(|err| log_warn!(err));
      }
    }


    #[cfg(feature = "auto_wake_lock")]
    if let Some(focus) = focus_change {
      if !focus {
        // release wake_lock
        wake_lock(&mut self.wake_lock, WakeLock::release);
      }
      else if app_ctx.auto_wake_lock {
        // request wake_lock
        wake_lock(&mut self.wake_lock, WakeLock::request);
      }
    }

    #[cfg(not(feature = "auto_wake_lock"))] {
      let _ = focus_change; // ignore unused warning
    }


    // animation
    #[cfg(feature = "frame_timer")] // detect state changes ... set control flow
    {
      if self.animate.note_change(&app_ctx.animate) {
        if app_ctx.animate {

          app_ctx.request = None;
          self.requested.set_state(None);

          #[cfg(feature = "auto_wake_lock")]
          if app_ctx.auto_wake_lock {
            // request wake_lock
            wake_lock(&mut self.wake_lock, WakeLock::request);
          }

          let now = Instant::now();

          if self.next <= now {
            self.next = now; // reset frame_timer
            app_ctx.window().request_redraw();
          }
          else {
            event_loop.set_earlier(self.next);
          }
        }
        else {
          #[cfg(feature = "auto_wake_lock")]
          // release wake_lock
          wake_lock(&mut self.wake_lock, WakeLock::release);

          event_loop.set_wait();
        }
      }

      if self.requested.changed(&app_ctx.request) {

        if app_ctx.animate {
          app_ctx.request = None;
          self.requested.set_state(None);
        }
        else if let Some(delay) = app_ctx.request {

          let earlier = match self.requested.state() {
            None => {
              self.requested.set_state(Some(delay));
              true // is definitely eralier
            },
            Some(later) if delay < *later => {
              self.requested.set_state(Some(delay));
              true // is earlier, checked
            },
            Some(previous) => {
              app_ctx.request = Some(*previous); // reset to previous
              false // is not earlier
            },
          };

          if earlier {
            if let Some(instant) = self.last.checked_add(delay) {

              let now = Instant::now();
              if self.next < now { self.next = now }

              if instant > self.next {
                event_loop.set_earlier(instant);
              }
              else {
                event_loop.set_earlier(self.next);
              }
            }
            // else consider as infinite delay, keep ControlFLow::Wait
          }
        }
        else {
          app_ctx.request = *self.requested.state(); // reset to previous
        }
      }
    }
  }
}