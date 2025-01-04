
use winit::{window::{WindowAttributes, Window, WindowId}, event::{WindowEvent, StartCause}, event_loop::ActiveEventLoop};
use winit::application::ApplicationHandler;
use std::sync::mpsc::{Receiver, sync_channel};

use crate::*;
use super::{AppHandler, AppState, AppCtx};


enum MountState<App: AppHandler> {
  Init {
    event_queue: Vec<PlatformEvent>,
    window_attributes: WindowAttributes,
    event_loop_proxy: PlatformEventLoopProxy,
    init_data: App::InitData,
  },
  Window {
    event_queue: Vec<PlatformEvent>,
    window: Window,
    event_loop_proxy: PlatformEventLoopProxy,
    init_data: App::InitData,
  },
  Mounting {
    event_queue: Vec<PlatformEvent>,
    window_id: WindowId,
    receiver: Receiver<AppState<App>>,
  },
  Mounted(AppState<App>),
}


pub struct AppMount<App: AppHandler> {
  state: MountState<App>,
}

impl<App: AppHandler> AppMount<App> {

  pub fn mount(event_loop_proxy: PlatformEventLoopProxy, window_attributes: WindowAttributes, init_data: App::InitData) -> Self {
    Self { state: MountState::Init { event_queue: Vec::new(), window_attributes, event_loop_proxy, init_data } }
  }

  pub fn run(self, event_loop: PlatformEventLoop) {

    #[cfg(not(target_family="wasm"))] {
      let mut app = self;
      event_loop.run_app(&mut app).unwrap();
    }

    #[cfg(target_family="wasm")] {
      use winit::platform::web::EventLoopExtWebSys;
      event_loop.spawn_app(self);
    }
  }

  pub fn start(window_attributes: WindowAttributes, init_data: App::InitData) {
    let event_loop = event_loop();
    Self::mount(event_loop.create_proxy(), window_attributes, init_data).run(event_loop);
  }

  pub fn event(&mut self, event: PlatformEvent, event_loop: &ActiveEventLoop) {

    match &mut self.state {

      // end state
      MountState::Mounted(app_state) => app_state.event(event, event_loop),

      // init state
      MountState::Init { event_queue, .. } => match &event {

        PlatformEvent::Resumed => {

          event_queue.push(event);

          take_mut::take(&mut self.state, |state| {
            if let MountState::Init { event_queue, window_attributes, init_data, event_loop_proxy } = state {

              let window = crate::window(event_loop, window_attributes);
              mount_window(&window);

              MountState::Window { window, event_queue, init_data, event_loop_proxy }
            }
            else { unreachable!() }
          });

        },

        _ => event_queue.push(event),
      },

      // after window creation
      MountState::Window { event_queue, window, .. } => match &event {

        PlatformEvent::WindowEvent { event: WindowEvent::Resized {..}, window_id: id } if *id == window.id() => {

          event_queue.push(event);

          take_mut::take(&mut self.state, |state| {
            if let MountState::Window { event_queue, window, init_data, event_loop_proxy } = state {

              let (sender, receiver) = sync_channel(1);
              let window_id = window.id();

              spawn_local(async move {
                let mut app_ctx = AppCtx::new(event_loop_proxy.clone(), window);
                let app = App::init(&mut app_ctx, init_data).await;
                let app_state = AppState::new(app_ctx, app);
                sender.send(app_state).unwrap();
                event_loop_proxy.send_event(PlatformEventExt::AppInit {window_id}).unwrap();
              });

              MountState::Mounting { event_queue, window_id, receiver }
            }
            else { unreachable!() }
          });
        },

        _ => event_queue.push(event),
      },

      // waiting for the app
      MountState::Mounting { event_queue, receiver, window_id } => match &event {

        PlatformEvent::UserEvent(PlatformEventExt::AppInit {window_id: id}) if id == window_id => {

          if let Ok(mut app_state) = receiver.try_recv() {

            for event in event_queue.drain(..) {
              app_state.event(event, event_loop);
            }

            self.state = MountState::Mounted(app_state);
          }
        },

        _ => event_queue.push(event),

      },

    }
  }
}


impl<App: AppHandler> ApplicationHandler<PlatformEventExt> for AppMount<App> {

  fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
    self.event(PlatformEvent::NewEvents(cause), event_loop);
  }

  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    self.event(PlatformEvent::Resumed, event_loop);
  }

  fn suspended(&mut self, event_loop: &ActiveEventLoop) {
    self.event(PlatformEvent::Suspended, event_loop);
  }

  fn user_event(&mut self, event_loop: &ActiveEventLoop, event: PlatformEventExt) {
    self.event(PlatformEvent::UserEvent(event), event_loop);
  }

  fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
    self.event(PlatformEvent::WindowEvent { window_id, event }, event_loop);
  }

}