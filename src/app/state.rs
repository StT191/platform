
use winit::{window::WindowId, dpi::PhysicalSize};
use super::{*};

#[cfg(feature = "auto_wake_lock")]
use crate::wake_lock::WakeLock;

#[cfg(feature = "frame_pacing")]
use crate::time::*;


pub(super) struct AppState<App: AppHandler> {
  #[cfg(feature = "conflate_resizes")] resized: Option<PhysicalSize<u32>>,
  #[cfg(feature = "auto_wake_lock")] wake_lock: Option<WakeLock>,
  #[cfg(feature = "auto_wake_lock")] auto_wake_lock: bool,
  #[cfg(feature = "auto_wake_lock")] occluded: bool,
  window_id: WindowId,
  pub(super) app_ctx: AppCtx<App::UserEvent>,
  app: App,
}

impl<App: AppHandler> AppState<App> {

  pub fn new(app_ctx: AppCtx<App::UserEvent>, app: App) -> Self {
    Self {
      #[cfg(feature = "conflate_resizes")] resized: None,

      #[cfg(feature = "auto_wake_lock")]
      wake_lock: WakeLock::new(
        #[cfg(target_os = "android")] app_ctx.android_app().clone()
      ).map_err(|m| log::warn!("{m:?}")).ok(),

      #[cfg(feature = "auto_wake_lock")] auto_wake_lock: false,
      #[cfg(feature = "auto_wake_lock")] occluded: false,

      window_id: app_ctx.window.id(),
      app_ctx, app,
    }
  }

  #[inline]
  pub fn event(&mut self, event: AppEvent<App::UserEvent>) {

    let app_ctx = &mut self.app_ctx;

    #[cfg(feature = "auto_wake_lock")]
    let mut focus_changed = false;

    match event {

      AppEvent::Resumed => {

        #[cfg(target_os = "android")] {
          app_ctx.android_state.suspended = false;
        }

        self.app.event(app_ctx, Event::Resumed);

        #[cfg(target_os = "android")] {

          let ((size, scale), state) = (
            app_ctx.android_state.window_changed(&app_ctx.window),
            app_ctx.android_state.state_changed(),
          );

          if size || scale || state {
            self.app.event(app_ctx, Event::WindowEvent(WindowEvent::Resized(app_ctx.android_state.window_size)));
          }

          app_ctx.request_redraw();
        }
      },

      AppEvent::Suspended => {

        self.app.event(app_ctx, Event::Suspended);

        #[cfg(target_os = "android")] {
          app_ctx.android_state.suspended = true;
        }
      },

      AppEvent::Exit => self.app.event(app_ctx, Event::Exit),

      #[cfg(feature = "futures")]
      AppEvent::FutureReady {id, output: ()} => {
        self.app.event(app_ctx, Event::FutureReady(id));
      },

      #[cfg(feature = "timeout")]
      AppEvent::Timeout {id: AppTimeoutId::User(id), instant} => {
        self.app.event(app_ctx, Event::Timeout {id, instant});
      },

      #[cfg(feature = "async_timeout")]
      AppEvent::Timeout {id: AppTimeoutId::Async(wake_id), ..} => {
        wake_id.wake();
        return;
      },

      #[cfg(feature = "frame_pacing")]
      AppEvent::Timeout {id: AppTimeoutId::FrameRequest(id), instant} if id == self.window_id => {
        app_ctx.frame_scheduled = false;
        app_ctx.request_redraw();
        app_ctx.frame_time = instant; // set to requested for best pacing
        app_ctx.frame_timeout = Some(instant);
        return;
      },

      AppEvent::UserEvent(AppEventExt::UserEvent(event)) => {
        self.app.event(app_ctx, Event::UserEvent(event));
      },

      #[cfg(feature = "device_events")]
      AppEvent::DeviceEvent {device_id, event} => {
        self.app.event(app_ctx, Event::DeviceEvent {device_id, event});
      },

      #[cfg(all(feature = "web_clipboard", target_family="wasm"))]
      AppEvent::UserEvent(AppEventExt::ClipboardFetch(id)) if id == self.window_id => {
        self.app.event(app_ctx, Event::ClipboardFetch);
      },

      #[cfg(all(feature = "web_clipboard", target_family="wasm"))]
      AppEvent::UserEvent(AppEventExt::ClipboardPaste(id)) if id == self.window_id  => {
        self.app.event(app_ctx, Event::ClipboardPaste);
      },

      AppEvent::WindowEvent {window_id: id, event: window_event} if id == self.window_id => {

        #[cfg(target_os = "android")]
        if app_ctx.android_state.inter_window_event(&window_event) {
          return; // wait for other events
        }

        // before user handler
        match &window_event {

          WindowEvent::RedrawRequested => {

            #[cfg(feature = "conflate_resizes")]
            if let Some(size) = self.resized.take() {
              self.app.event(app_ctx, Event::WindowEvent(WindowEvent::Resized(size)));
            }

            #[cfg(feature = "frame_pacing")] {

              if !app_ctx.redraw_requested {
                app_ctx.frame_time = Instant::now();
              }

              app_ctx.cancel_frame(); // only if another one was scheduled
            }

            app_ctx.redraw_requested = false;
          },

          #[allow(unused)]
          WindowEvent::Resized(size) => {
            app_ctx.request_redraw();

            #[cfg(feature = "conflate_resizes")] {
              self.resized = Some(*size);
              return;
            }
          },

          WindowEvent::ScaleFactorChanged {..} => {
            app_ctx.request_redraw();
          },

          #[cfg(feature = "frame_pacing")]
          WindowEvent::Moved(_) => {
            app_ctx.fetch_monitor_frame_duration();
          },

          WindowEvent::CloseRequested => {
            app_ctx.exit = true;
          },

          #[cfg(feature = "auto_wake_lock")]
          WindowEvent::Focused(_) => {
            focus_changed = true;
          },

          #[cfg(feature = "auto_wake_lock")]
          WindowEvent::Occluded(occluded) => { // occluded is more reliable on web
            self.occluded = *occluded;
            focus_changed = true;
          },

          _ => {},
        }

        // exec event handler
        self.app.event(app_ctx, Event::WindowEvent(window_event));
      },

      _ => return,
    }


    #[cfg(feature = "auto_wake_lock")]
    if focus_changed || self.auto_wake_lock.note_change(&app_ctx.auto_wake_lock) {

      let visible = app_ctx.window.has_focus() && !self.occluded;

      if !visible || !app_ctx.auto_wake_lock {
        // release wake_lock
        self.wake_lock.as_mut().map(|lock| lock.release().map_err(|m| log::warn!("{m:?}")));
      }
      else if app_ctx.auto_wake_lock && visible {
        // request wake_lock
        self.wake_lock.as_mut().map(|lock| lock.request().map_err(|m| log::warn!("{m:?}")));
      }

      self.auto_wake_lock = app_ctx.auto_wake_lock;
    }


    #[cfg(feature = "frame_pacing")]
    if app_ctx.schedule_frame.take() {

      let instant = app_ctx.frame_timeout.expect("frame_timeout needs to be set when scheduling a frame");

      if instant > Instant::now() {
        let _ = app_ctx.timer.borrow_mut().set_timeout_earlier(
          AppTimeoutId::FrameRequest(self.window_id),
          instant,
        );
        app_ctx.frame_scheduled = true;
      } else {
        app_ctx.request_redraw();
      }
    }

  }

}