
use winit::{window::WindowId, event::WindowEvent, event_loop::ActiveEventLoop};
use crate::*;

#[cfg(feature = "redraw_timer")]
use crate::time::*;

#[cfg(feature = "redraw_timer")]
use winit::event::StartCause;

#[cfg(feature = "auto_wake_lock")]
use crate::wake_lock::WakeLock;

use super::{AppEvent, AppCtx, AppHandler};


pub(super) struct AppState<App: AppHandler> {
  #[cfg(feature = "auto_wake_lock")] wake_lock: Option<WakeLock>,
  #[cfg(feature = "redraw_timer")] requested_redraw: DetectChanges<Option<Instant>>,
  window_id: WindowId,
  app_ctx: AppCtx,
  app: App,
}

impl<App: AppHandler> AppState<App> {

  pub(super) fn new(app_ctx: AppCtx, app: App) -> Self {
    Self {
      #[cfg(feature = "auto_wake_lock")] wake_lock: WakeLock::new().map_err(|m| log::warn!("{m:?}")).ok(),
      #[cfg(feature = "redraw_timer")] requested_redraw: DetectChanges::new(None),
      window_id: app_ctx.window().id(),
      app_ctx, app,
    }
  }

  pub(super) fn event(&mut self, event: PlatformEvent, event_loop: &ActiveEventLoop) {

    let app_ctx = &mut self.app_ctx;

    match event {

      #[cfg(feature = "redraw_timer")]
      PlatformEvent::NewEvents(StartCause::ResumeTimeReached {..}) => {
        app_ctx.window().request_redraw();
        event_loop.set_wait();
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

      PlatformEvent::DeviceEvent {device_id, event} => {
        self.app.event(app_ctx, &AppEvent::DeviceEvent {device_id, event});
        self.after_event(event_loop, None);
      },

      PlatformEvent::WindowEvent { window_id: id, event: window_event } if id == self.window_id => {

        #[cfg(feature = "auto_wake_lock")]
        let mut focus_change: Option<bool> = None;

        // before user handler
        match &window_event {

          #[cfg(feature = "redraw_timer")]
          WindowEvent::RedrawRequested => {
            app_ctx.redraw_time = None;
            self.requested_redraw.set_state(None);
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

    // animation
    #[cfg(feature = "redraw_timer")] // detect state changes ... set control flow
    if self.requested_redraw.note_change(&app_ctx.redraw_time) {
      if let Some(redraw_time) = app_ctx.redraw_time {
        event_loop.set_wait_until(redraw_time);
      } else {
        event_loop.set_wait();
      }
    }

  }
}