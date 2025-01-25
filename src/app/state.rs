
use winit::{window::WindowId, event::*, event_loop::ActiveEventLoop};
use super::{*, Event};

#[cfg(feature = "auto_wake_lock")]
use crate::wake_lock::WakeLock;

#[cfg(feature = "frame_pacing")]
use crate::time::*;


pub(super) struct AppState<App: AppHandler> {
  #[cfg(feature = "frame_pacing")] frame_requested: DetectChanges<bool>,
  #[cfg(all(feature = "frame_pacing", not(target_family = "wasm")))] redraw_requested: bool,
  #[cfg(feature = "auto_wake_lock")] wake_lock: Option<WakeLock>,
  window_id: WindowId,
  app_ctx: AppCtx,
  app: App,
}

impl<App: AppHandler> AppState<App> {

  pub(super) fn new(app_ctx: AppCtx, app: App) -> Self {
    Self {
      #[cfg(feature = "frame_pacing")] frame_requested: DetectChanges::new(false),
      #[cfg(all(feature = "frame_pacing", not(target_family = "wasm")))] redraw_requested: false,
      #[cfg(feature = "auto_wake_lock")] wake_lock: WakeLock::new().map_err(|m| log::warn!("{m:?}")).ok(),
      window_id: app_ctx.window.id(),
      app_ctx, app,
    }
  }

  pub(super) fn event(&mut self, event: AppEvent, event_loop: &ActiveEventLoop) {

    let app_ctx = &mut self.app_ctx;

    match event {

      AppEvent::Resumed => {
        self.app.event(app_ctx, &Event::Resumed);
        self.after_event(event_loop, None);
      },

      AppEvent::Suspended => {
        self.app.event(app_ctx, &Event::Suspended);
        self.after_event(event_loop, None);
      },

      #[cfg(feature = "app_waker")]
      AppEvent::UserEvent(AppEventExt::Wake { window_id: id, wake_id }) if id == self.window_id  => {
        self.app.event(app_ctx, &Event::Wake(wake_id));
        self.after_event(event_loop, None);
      },

      #[cfg(all(feature = "web_clipboard", target_family="wasm"))]
      AppEvent::UserEvent(AppEventExt::ClipboardFetch { window_id: id }) if id == self.window_id => {
        self.app.event(app_ctx, &Event::ClipboardFetch);
        self.after_event(event_loop, None);
      },

      #[cfg(all(feature = "web_clipboard", target_family="wasm"))]
      AppEvent::UserEvent(AppEventExt::ClipboardPaste { window_id: id }) if id == self.window_id  => {
        self.app.event(app_ctx, &Event::ClipboardPaste);
        self.after_event(event_loop, None);
      },

      #[cfg(feature = "device_events")]
      AppEvent::DeviceEvent {device_id, event} => {
        self.app.event(app_ctx, &Event::DeviceEvent {device_id, event});
        self.after_event(event_loop, None);
      },

      #[cfg(any(
        feature = "app_timer",
        all(feature = "frame_pacing", not(target_family = "wasm")),
      ))]
      AppEvent::NewEvents(StartCause::ResumeTimeReached {..}) => {

        if let Some(timeout) = app_ctx.timer.pop_timeout() {
          match timeout.id {
            #[cfg(feature = "app_timer")]
            timer::TimerId::User(id) => {
              self.app.event(app_ctx, &Event::Timeout { instant: timeout.instant, id });
            },
            #[cfg(all(feature = "frame_pacing", not(target_family = "wasm")))]
            timer::TimerId::FrameRequest => {
              app_ctx.frame_time = timeout.instant;
              app_ctx.window.request_redraw();
              self.redraw_requested = true;
            },
          }
        }

        self.app_ctx.timer.shrink_queue();

        self.after_event(event_loop, None);
      },

      AppEvent::WindowEvent { window_id: id, event: window_event } if id == self.window_id => {

        #[cfg(feature = "auto_wake_lock")]
        let mut focus_change: Option<bool> = None;

        // before user handler
        match &window_event {

          #[cfg(feature = "frame_pacing")]
          WindowEvent::RedrawRequested => {

            #[cfg(target_family = "wasm")] {
              app_ctx.frame_time = Instant::now();
            }

            #[cfg(not(target_family = "wasm"))] {
              if *self.frame_requested.state() {
                // frame_request-timeout is still in progress, wait till finished
                if !self.redraw_requested { return; }
              }
              else {
                app_ctx.frame_time = Instant::now();
              }

              self.redraw_requested = false;
            }

            app_ctx.frame_request = false;
            self.frame_requested.set_state(false);
          },

          WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged {..} => {
            app_ctx.window.request_redraw();
          },

          #[cfg(all(feature = "frame_pacing", not(target_family = "wasm")))]
          WindowEvent::Moved(_) => {
            app_ctx.fetch_monitor_frame_duration();
          },

          WindowEvent::CloseRequested => {
            app_ctx.exit = true;
          },

          #[cfg(feature = "auto_wake_lock")]
          WindowEvent::Focused(focus) => { focus_change = Some(*focus) },

          _ => {},
        }

        // exec event handler
        self.app.event(app_ctx, &Event::WindowEvent(window_event));

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


    #[cfg(feature = "frame_pacing")]
    if self.frame_requested.note_change(&app_ctx.frame_request) {
      #[cfg(target_family = "wasm")] {
        app_ctx.frame_request = false;
        app_ctx.window.request_redraw();
      }
      #[cfg(not(target_family = "wasm"))] {

        let instant = app_ctx.frame_time + app_ctx.frame_duration;
        let now = Instant::now();

        if instant > now {
          app_ctx.timer.set_timeout(timer::Timeout {instant, id: timer::TimerId::FrameRequest}, true);
        } else {
          app_ctx.frame_time = now;
          app_ctx.window.request_redraw();
          self.redraw_requested = true;
        }
      }
    }


    #[cfg(any(
      feature = "app_timer",
      all(feature = "frame_pacing", not(target_family = "wasm")),
    ))]
    if let Some(set_instant) = app_ctx.timer.take_set_instant() {
      match set_instant {
        Some(instant) => event_loop.set_wait_until(instant),
        None => event_loop.set_wait(),
      }
    }


    #[cfg(feature = "auto_wake_lock")]
    if let Some(focus) = focus_change {
      if !focus {
        // release wake_lock
        self.wake_lock.as_mut().map(|lock| lock.release().map_err(|m| log::warn!("{m:?}")));
      }
      else if app_ctx.auto_wake_lock {
        // request wake_lock
        self.wake_lock.as_mut().map(|lock| lock.request().map_err(|m| log::warn!("{m:?}")));
      }
    }

    #[cfg(not(feature = "auto_wake_lock"))] {
      let _ = focus_change; // ignore unused warning
    }

  }
}