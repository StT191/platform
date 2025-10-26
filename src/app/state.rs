
use winit::window::WindowId;
use super::{*};

#[cfg(feature = "auto_wake_lock")]
use crate::wake_lock::WakeLock;

#[cfg(feature = "frame_pacing")]
use crate::time::*;


pub(super) struct AppState<App: AppHandler> {
  #[cfg(feature = "frame_pacing")] redraw_requested: bool,
  #[cfg(feature = "auto_wake_lock")] wake_lock: Option<WakeLock>,
  #[cfg(feature = "auto_wake_lock")] auto_wake_lock: DetectChanges<bool>,
  window_id: WindowId,
  pub(super) app_ctx: AppCtx<App::UserEvent>,
  app: App,
}

impl<App: AppHandler> AppState<App> {

  pub fn new(app_ctx: AppCtx<App::UserEvent>, app: App) -> Self {
    Self {
      #[cfg(feature = "frame_pacing")] redraw_requested: false,
      #[cfg(feature = "auto_wake_lock")] wake_lock: WakeLock::new().map_err(|m| log::warn!("{m:?}")).ok(),
      #[cfg(feature = "auto_wake_lock")] auto_wake_lock: DetectChanges::new(false),

      window_id: app_ctx.window.id(),
      app_ctx, app,
    }
  }

  pub fn event(&mut self, event: AppEvent<App::UserEvent>) {

    let app_ctx = &mut self.app_ctx;

    match event {

      AppEvent::Resumed => {
        self.app.event(app_ctx, Event::Resumed);
        self.after_event(None);
      },

      AppEvent::Suspended => {
        self.app.event(app_ctx, Event::Suspended);
        self.after_event(None);
      },

      #[cfg(feature = "futures")]
      AppEvent::FutureReady {id, output: ()} => {
        self.app.event(app_ctx, Event::FutureReady(id));
        self.after_event(None);
      },

      #[cfg(feature = "timeout")]
      AppEvent::Timeout {id: AppTimeoutId::User(id), instant} => {
        self.app.event(app_ctx, Event::Timeout {id, instant});
        self.after_event(None);
      },

      #[cfg(feature = "async_timeout")]
      AppEvent::Timeout {id: AppTimeoutId::Async(wake_id), ..} => wake_id.wake(),

      #[cfg(feature = "frame_pacing")]
      AppEvent::Timeout {id: AppTimeoutId::FrameRequest(id), instant} if id == self.window_id => {
        app_ctx.frame_time = instant;
        app_ctx.window.request_redraw();
        self.redraw_requested = true;
      },

      AppEvent::UserEvent(AppEventExt::UserEvent(event)) => {
        self.app.event(app_ctx, Event::UserEvent(event));
        self.after_event(None);
      },

      #[cfg(feature = "device_events")]
      AppEvent::DeviceEvent {device_id, event} => {
        self.app.event(app_ctx, Event::DeviceEvent {device_id, event});
        self.after_event(None);
      },

      #[cfg(all(feature = "web_clipboard", target_family="wasm"))]
      AppEvent::UserEvent(AppEventExt::ClipboardFetch(id)) if id == self.window_id => {
        self.app.event(app_ctx, Event::ClipboardFetch);
        self.after_event(None);
      },

      #[cfg(all(feature = "web_clipboard", target_family="wasm"))]
      AppEvent::UserEvent(AppEventExt::ClipboardPaste(id)) if id == self.window_id  => {
        self.app.event(app_ctx, Event::ClipboardPaste);
        self.after_event(None);
      },

      AppEvent::WindowEvent {window_id: id, event: window_event} if id == self.window_id => {

        #[cfg(feature = "auto_wake_lock")]
        let mut focus_change: Option<bool> = None;

        // before user handler
        match &window_event {

          #[cfg(feature = "frame_pacing")]
          WindowEvent::RedrawRequested => {

            if app_ctx.frame_timeout.is_none() || !self.redraw_requested {
              // no frame was requested or frame_request-timeout is still in progress
              app_ctx.frame_time = Instant::now();
            }

            self.redraw_requested = false;
            app_ctx.frame_timeout = None;
          },

          WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged {..} => {
            app_ctx.window.request_redraw();
          },

          #[cfg(feature = "frame_pacing")]
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
        self.app.event(app_ctx, Event::WindowEvent(window_event));

        self.after_event({
          #[cfg(feature = "auto_wake_lock")] { focus_change }
          #[cfg(not(feature = "auto_wake_lock"))] { None }
        });
      },

      _ => {}

    }
  }

  fn after_event(&mut self, focus_change: Option<bool>) {

    #[allow(unused)]
    let app_ctx = &mut self.app_ctx;

    #[cfg(feature = "frame_pacing")]
    if app_ctx.schedule_frame {

      app_ctx.schedule_frame = false;

      let instant = app_ctx.frame_timeout.expect("frame_timeout needs to be set when scheduling a frame");
      let now = Instant::now();

      if instant > now {
        let _ = app_ctx.timer.borrow_mut().set_timeout_earlier(
          AppTimeoutId::FrameRequest(self.window_id),
          instant
        );
      } else {
        app_ctx.frame_time = now;
        app_ctx.window.request_redraw();
        self.redraw_requested = true;
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
      self.auto_wake_lock.set_state(app_ctx.auto_wake_lock);
    }
    else if self.auto_wake_lock.note_change(&app_ctx.auto_wake_lock) {
      if app_ctx.auto_wake_lock && app_ctx.window.has_focus() {
        // request wake_lock
        self.wake_lock.as_mut().map(|lock| lock.request().map_err(|m| log::warn!("{m:?}")));
      } else {
        // release wake_lock
        self.wake_lock.as_mut().map(|lock| lock.release().map_err(|m| log::warn!("{m:?}")));
      }
    }

    #[cfg(not(feature = "auto_wake_lock"))] {
      let _ = focus_change; // ignore unused warning
    }

  }
}