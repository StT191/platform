
use winit::{window::{WindowAttributes, Window}, event::*, event_loop::ActiveEventLoop};
use std::{mem::replace, sync::mpsc::{Receiver, sync_channel}};

use crate::*;
use super::*;

enum MountState<App: AppHandler> {
  Init {
    event_queue: Vec<AppEvent<App::UserEvent>>,
    window_attributes: WindowAttributes,
    init_data: App::InitData,
  },
  Window {
    event_queue: Vec<AppEvent<App::UserEvent>>,
    window: Window,
    init_data: App::InitData,
  },
  Mounting {
    event_queue: Vec<AppEvent<App::UserEvent>>,
    future_id: AppFutureId,
    receiver: Receiver<AppState<App>>,
  },
  Mounted(AppState<App>),
  Empty,
}


pub struct AppMount<App: AppHandler> {
  state: MountState<App>,
}


impl<App: AppHandler> AppMount<App> {
  pub fn new(window_attributes: WindowAttributes, init_data: App::InitData) -> Self {
    Self { state: MountState::Init { event_queue: Vec::new(), window_attributes, init_data } }
  }

  fn take(&mut self) -> MountState<App> {
    replace(&mut self.state, MountState::Empty)
  }
}


impl<App: AppHandler> Runtime for AppMount<App> {

  type FutureId = AppFutureId;
  type Futures = AppFutures;
  type UserEvent = AppEventExt<App::UserEvent>;
  type TimeoutId = AppTimeoutId;

  fn event(&mut self, event_loop: &ActiveEventLoop, ctx: &mut AppRuntimeCtx<App::UserEvent>, event: AppEvent<App::UserEvent>) {

    if matches!(event, AppEvent::Exit) {
      self.state = MountState::Empty;
    };

    match &mut self.state {

      // end state
      MountState::Mounted(app_state) => {
        app_state.event(event);
        if app_state.app_ctx.exit {
          event_loop.exit();
        }
      },

      // init state
      MountState::Init {event_queue, ..} => match &event {

        AppEvent::Resumed => {

          event_queue.push(event);

          let MountState::Init {event_queue, window_attributes, init_data} = self.take() else { unreachable!() };

          let window = crate::window(event_loop, window_attributes);
          mount_window(&window);

          self.state = MountState::Window {window, event_queue, init_data};
        },

        _ => event_queue.push(event),
      },

      // after window creation
      MountState::Window {event_queue, ..} => match &event {

        AppEvent::WindowEvent {event: WindowEvent::Resized {..}, ..} => {

          event_queue.push(event);

          let MountState::Window {event_queue, window, init_data} = self.take() else { unreachable!() };

          let (sender, receiver) = sync_channel(1);
          let futures = ctx.futures.clone();
          let timer = ctx.timer.clone();
          let event_dispatcher = ctx.event_dispatcher.clone();

          let future_id = ctx.futures.spawn(Box::pin(async move {
            let mut app_ctx = AppCtx::new(futures, timer, event_dispatcher, window);
            let app = App::init(&mut app_ctx, init_data).await;
            let app_state = AppState::new(app_ctx, app);
            sender.send(app_state).unwrap();
          }));

          self.state = MountState::Mounting {event_queue, receiver, future_id};
        },

        _ => event_queue.push(event),
      },

      // waiting for the app
      MountState::Mounting {event_queue, receiver, future_id} => match &event {

        AppEvent::FutureReady {id, ..} => if id == future_id {

          let mut app_state = receiver.recv().unwrap();

          for event in event_queue.drain(..) {

            app_state.event(event);

            // exit as soon as possible
            if app_state.app_ctx.exit {
              event_loop.exit();
              return;
            }
          }

          self.state = MountState::Mounted(app_state);
        },

        _ => event_queue.push(event),
      },

      // dropped, ignore further events
      MountState::Empty => (),
    }
  }

}
