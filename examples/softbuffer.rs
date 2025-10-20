
use platform::winit::{
  window::WindowAttributes, event::{WindowEvent, KeyEvent, ElementState}, keyboard::{PhysicalKey, KeyCode},
  dpi::PhysicalSize,
};
use platform::{*};


main_app_closure! {
    LogLevel::Warn,
    WindowAttributes::default().with_inner_size(PhysicalSize::new(1000, 1000)),
    init_app,
}


async fn init_app(ctx: &mut AppCtx) -> impl FnMut(&mut AppCtx, Event) + use<> {


    // fill window-background with softbuffer
    let window = ctx.window_clone();

    let context = softbuffer::Context::new(window.clone()).unwrap();

    let mut surface = softbuffer::Surface::new(&context, window.clone()).unwrap();
    let size = window.inner_size();
    surface.resize(size.width.try_into().unwrap(), size.height.try_into().unwrap()).unwrap();

    let bg_color = [0x00, 0xff, 0x00, 0x00];


    ctx.set_frame_duration(Some(time::Duration::from_secs_f64(1.0/30.0)));


    let mut frame_counter = timer::IntervalCounter::from_secs(3.0);

    move |ctx, event| match event {

        Event::WindowEvent(WindowEvent::Resized(size)) => {
            surface.resize(size.width.try_into().unwrap(), size.height.try_into().unwrap()).unwrap();
        },

        Event::WindowEvent(WindowEvent::KeyboardInput { event: KeyEvent {
            state: ElementState::Pressed, physical_key: PhysicalKey::Code(KeyCode::KeyR), ..
        }, ..}) => {
            ctx.schedule_frame(time::Instant::now());
        },

        Event::WindowEvent(WindowEvent::RedrawRequested) => {

            // let then = time::Instant::now();

            let mut buffer = surface.buffer_mut().unwrap();

            buffer.fill(u32::from_be_bytes(bg_color));

            buffer.present().unwrap();

            // window.pre_present_notify();
            // window.request_redraw();
            // ctx.schedule_frame(time::Instant::now());
            ctx.request_frame();

            frame_counter.add();
            if let Some(counted) = frame_counter.count() { log::warn!("{counted:?}") }

            // println!("{:?}", then.elapsed());
        },

        _ => {}
    }
}