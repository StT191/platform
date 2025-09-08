
use winit::event_loop::{EventLoop, ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowAttributes};

#[cfg(target_family="wasm")]
use winit::platform::web::{WindowExtWebSys, WindowAttributesExtWebSys};

use crate::LogLevel;


pub fn init(log_level: LogLevel) {
    #[cfg(not(target_family="wasm"))] {
        simple_logger::init_with_level(log_level).unwrap();
    }

    #[cfg(target_family="wasm")] {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log_level).expect("could not initialize logger");
    }
}


pub fn event_loop<T>() -> EventLoop<T> {
    EventLoop::with_user_event().build().unwrap()
}


pub fn window(event_loop: &ActiveEventLoop, window_attributes: WindowAttributes) -> Window {

    #[cfg(not(target_family="wasm"))] {
        event_loop.create_window(window_attributes).unwrap()
    }

    #[cfg(target_family="wasm")] {
        event_loop.create_window(window_attributes.with_prevent_default(false)).unwrap()
    }
}


#[allow(unused_variables)]
pub fn mount_window(window: &Window) {
    #[cfg(target_family="wasm")] {

        // web
        let web_window = web_sys::window().unwrap();

        let body = web_window.document().and_then(|document| document.body()).unwrap();

        // remove previous elements
        while let Some(child) = body.last_child() {
            body.remove_child(&child).unwrap();
        }

        // set css styles
        body.set_attribute("style", "margin: 0; overflow: hidden;").unwrap();

        // append canvas to body
        let canvas_element = web_sys::HtmlElement::from(window.canvas().unwrap());

        canvas_element.set_attribute("style", "
            touch-action: none; outline: none; width: 100%; height: 100%;
            position: absolute; top: 0; bottom: 0; left: 0; right: 0;
        ").unwrap();

        body.append_child(&canvas_element).unwrap();

        canvas_element.focus().unwrap(); // initial focus
    }
}


// extension

use crate::time::{Instant, Duration};

pub trait ControlFlowExtension {
    fn set_poll(&self);
    fn set_wait(&self);
    fn set_wait_until(&self, instant: Instant);
    fn set_earlier(&self, instant: Instant);
}

impl ControlFlowExtension for ActiveEventLoop {

    fn set_poll(&self) { self.set_control_flow(ControlFlow::Poll); }
    fn set_wait(&self) { self.set_control_flow(ControlFlow::Wait); }
    fn set_wait_until(&self, instant: Instant) { self.set_control_flow(ControlFlow::WaitUntil(instant)); }

    fn set_earlier(&self, instant: Instant) {
        match self.control_flow() {
            ControlFlow::Poll => {},
            ControlFlow::Wait => self.set_wait_until(instant),
            ControlFlow::WaitUntil(other) => self.set_wait_until(instant.min(other)),
        }
    }
}


pub trait WindowExtFrameDuration {
    fn frame_duration(&self) -> Option<Duration>;
}

impl WindowExtFrameDuration for Window {
    fn frame_duration(&self) -> Option<Duration> {

        // try current monitor
        let rt_hz = self.current_monitor().and_then(|m| m.refresh_rate_millihertz());

        // try if there is only one monitor
        let rt_hz = rt_hz.or_else(|| {

            let mut count = 0;
            let mut rt_hz = None;

            for monitor in self.available_monitors() {

                count += 1;
                let this_rt_hz = monitor.refresh_rate_millihertz();

                if this_rt_hz.is_none() || count > 1 && this_rt_hz != rt_hz {
                    return None;
                }
                else {
                    rt_hz = this_rt_hz;
                }
            }

            rt_hz
        })?;

        Some(Duration::from_nanos(10_u64.pow(12) / rt_hz as u64))
    }
}