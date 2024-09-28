
use winit::window::Window;
use wgx::{WgxDevice, WgxDeviceQueue, RenderTarget, wgpu::{CommandEncoder, RenderPass}};
pub use egui_wgpu::{Renderer, ScreenDescriptor};
use epaint::{Rect, ClippedPrimitive, textures::TexturesDelta};


// helper trait
pub trait ScreenDescriptorExtension {
  fn new(size_in_pixels: [u32; 2], pixels_per_point: f32) -> Self;
  fn from_window(window: &Window) -> Self;
  fn clone(&self) -> Self;
  fn clip_rect(&self) -> Rect;
}


pub fn renderer(gx: &impl WgxDevice, target: &impl RenderTarget) -> Renderer {
  Renderer::new(gx.device(), target.view_format(), target.depth_testing(), target.msaa(), false)
}


fn prepare_renderer(
  renderer: &mut Renderer, gx: &impl WgxDeviceQueue, encoder: &mut CommandEncoder,
  textures_delta: &TexturesDelta, clipped_primitives: &[ClippedPrimitive], screen_dsc: &ScreenDescriptor,
) {
  for (id, image_delta) in &textures_delta.set {
    renderer.update_texture(gx.device(), gx.queue(), *id, image_delta);
  }

  if !clipped_primitives.is_empty() {
    let commands = renderer.update_buffers(gx.device(), gx.queue(), encoder, clipped_primitives, screen_dsc);

    if !commands.is_empty() {
      gx.queue().submit(commands);
    }
  }

  for id in &textures_delta.free {
    renderer.free_texture(id);
  }
}


// epaint drawing context

mod epaint_ctx {

  use super::*;
  use epaint::{
    Fonts, text::{FontDefinitions, /*FontId*/}, TextureId, Shape, ClippedShape, ClippedPrimitive,
    Tessellator, TessellationOptions, ImageData, TextureManager, TextureAtlas,
  };


  pub fn clip_shapes(shapes: impl IntoIterator<Item=Shape>, clip_rect: Rect) -> impl Iterator<Item=ClippedShape> {
    shapes.into_iter().map(move |shape| ClippedShape {shape, clip_rect})
  }


  pub struct EpaintCtx {
    pub texture_manager: TextureManager,
    pub fonts: Fonts,
    pub screen_dsc: ScreenDescriptor,
  }


  impl EpaintCtx {

    pub fn new(screen_dsc: ScreenDescriptor, max_texture_side: usize, font_defs: FontDefinitions) -> Self {

      let mut texture_manager = TextureManager::default();

      let fonts = Fonts::new(screen_dsc.pixels_per_point, max_texture_side, font_defs);

      assert_eq!(
        texture_manager.alloc("font_texture".to_string(), ImageData::Font(fonts.image()), TextureAtlas::texture_options()),
        TextureId::default(),
      );

      Self { texture_manager, fonts, screen_dsc }
    }

    pub fn begin_frame(&mut self, screen_dsc: Option<ScreenDescriptor>, max_texture_side: Option<usize>) {
      if let Some(screen_dsc) = screen_dsc {
        self.screen_dsc = screen_dsc;
      }
      let max_texture_side = max_texture_side.unwrap_or(self.fonts.max_texture_side());
      self.fonts.begin_pass(self.screen_dsc.pixels_per_point, max_texture_side);
    }

    pub fn clip_shapes(&self, shapes: impl IntoIterator<Item=Shape>, clip_rect: Option<Rect>) -> impl Iterator<Item=ClippedShape> {
      clip_shapes(shapes, clip_rect.unwrap_or(self.screen_dsc.clip_rect()))
    }

    pub fn tessellate(&self, options: TessellationOptions, shapes: impl IntoIterator<Item=ClippedShape>, out: &mut Vec<ClippedPrimitive>) {

      let mut tesselator = Tessellator::new(
        self.screen_dsc.pixels_per_point, options, self.fonts.image().size,
        self.fonts.texture_atlas().lock().prepared_discs(),
      );

      for clipped_shape in shapes {
        tesselator.tessellate_clipped_shape(clipped_shape, out);
      }
    }

    pub fn prepare(&mut self,
      renderer: &mut Renderer, gx: &impl WgxDeviceQueue, encoder: &mut CommandEncoder,
      clipped_primitives: &[ClippedPrimitive],
    ) {
      // update fonts texture it necessary
      if let Some(image_delta) = self.fonts.texture_atlas().lock().take_delta() {
        self.texture_manager.set(TextureId::default(), image_delta);
      }
      prepare_renderer(renderer, gx, encoder, &self.texture_manager.take_delta(), clipped_primitives, &self.screen_dsc);
    }

    pub fn render<'a>(&'a self,
      renderer: &'a Renderer, rpass: &mut RenderPass<'static>, clipped_primitives: &'a [ClippedPrimitive],
    ) {
      renderer.render(rpass, clipped_primitives, &self.screen_dsc);
    }
  }
}

pub use epaint_ctx::*;



// egui context

#[cfg(feature = "egui")]
#[allow(clippy::module_inception)]
mod egui_ctx {

  use epaint::ahash::HashSet;

  use super::*;
  use winit::event::WindowEvent;
  use crate::{time::Duration, AppCtx, AppEvent};

  #[cfg(all(feature = "web_clipboard", target_family="wasm"))]
  use crate::{web_clipboard::WebClipboard, log_warn};

  use egui::{Context, ClippedPrimitive, TexturesDelta, ViewportCommand, ViewportInfo, ViewportId};
  use egui_winit::{State, update_viewport_info, process_viewport_commands};


  pub struct EguiCtx {
    pub context: Context,
    pub state: State,
    pub screen_dsc: ScreenDescriptor,

    #[cfg(all(feature = "web_clipboard", target_family="wasm"))]
    pub web_clipboard: WebClipboard,
  }

  pub struct FrameOutput {
    pub clipped_primitives: Vec<ClippedPrimitive>,
    pub textures_delta: TexturesDelta,
    pub screen_dsc: ScreenDescriptor,
    pub commands: Vec<ViewportCommand>,
    pub repaint_delay: Duration,
  }


  impl EguiCtx {

    pub fn new(app_ctx: &AppCtx) -> Self {
      let context = Context::default();
      // install image loaders, need to be added via features in egui_extras
      egui_extras::install_image_loaders(&context);

      let screen_dsc = ScreenDescriptor::from_window(app_ctx.window());

      #[allow(unused_mut)]
      let mut state = State::new(
        context.clone(), ViewportId::ROOT, app_ctx.window(),
        Some(screen_dsc.pixels_per_point), None /* theme */, None /* max_texture_side */,
      );

      #[cfg(not(all(feature = "web_clipboard", target_family="wasm")))] {
        Self { state, screen_dsc, context }
      }

      #[cfg(all(feature = "web_clipboard", target_family="wasm"))] {

        state.set_clipboard_text("DUMMY_CONTENT".to_string());

        let web_clipboard = WebClipboard::connect(app_ctx, true);
        log_warn!(web_clipboard);

        Self { state, screen_dsc, context, web_clipboard }
      }
    }

    pub fn event(&mut self, app_ctx: &AppCtx, app_event: &AppEvent) -> (bool, bool) {
      match app_event {

        AppEvent::WindowEvent(window_event) => {

          if matches!(window_event, WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged {..}) {
            self.screen_dsc = ScreenDescriptor::from_window(app_ctx.window());
          }

          if *window_event != WindowEvent::RedrawRequested {
            let res = self.state.on_window_event(app_ctx.window(), window_event);

            #[cfg(all(feature = "web_clipboard", target_family="wasm"))] {
              let events = &mut self.state.egui_input_mut().events;

              if let Some(egui::Event::Paste(_)) = events.last() {
                // self.web_clipboard.fetch(); // disabled: listening to web-canvas paste event instead
                events.pop();
                return (false, false)
              }
            }

            return (res.repaint, res.consumed)
          }

          (false, false)
        },

        #[cfg(all(feature = "web_clipboard", target_family="wasm"))]
        AppEvent::ClipboardPaste => {
          if let Some(text) = self.web_clipboard.read() {
            self.state.egui_input_mut().events.push(egui::Event::Paste(text));
            (true, true)
          }
          else {(false, false)}
        },

        _ => (false, false),
      }
    }

    pub fn run(&mut self, app_ctx: &AppCtx, ui_fn: impl FnMut(&Context)) -> FrameOutput {

      let mut input = self.state.take_egui_input(app_ctx.window());

      let viewport_id = input.viewport_id;

      let viewport_info = input.viewports.get_mut(&viewport_id).unwrap();
      update_viewport_info(viewport_info, &self.context, app_ctx.window(), false);

      let mut output = self.context.run(input, ui_fn);

      #[cfg(all(feature = "web_clipboard", target_family="wasm"))]
      if !output.platform_output.copied_text.is_empty() {
        let copied = std::mem::take(&mut output.platform_output.copied_text);
        self.web_clipboard.write(copied);
      }

      self.state.handle_platform_output(app_ctx.window(), output.platform_output);

      let viewport_output = output.viewport_output.remove(&viewport_id).unwrap();

      if !viewport_output.commands.is_empty() {
        process_viewport_commands(
          &self.context,
          &mut ViewportInfo::default(),
          viewport_output.commands.iter().cloned(),
          app_ctx.window(),
          &mut HashSet::default(),
        );
      }

      FrameOutput {
        clipped_primitives: self.context.tessellate(output.shapes, output.pixels_per_point),
        textures_delta: output.textures_delta,
        screen_dsc: self.screen_dsc.clone(),
        commands: viewport_output.commands,
        repaint_delay: viewport_output.repaint_delay,
      }
    }
  }


  impl FrameOutput {

    pub fn prepare(&self, renderer: &mut Renderer, gx: &impl WgxDeviceQueue, encoder: &mut CommandEncoder) {
      prepare_renderer(renderer, gx, encoder, &self.textures_delta, &self.clipped_primitives, &self.screen_dsc);
    }

    pub fn render<'a>(&'a self, renderer: &'a Renderer, rpass: &mut RenderPass<'static>) {
      renderer.render(rpass, &self.clipped_primitives, &self.screen_dsc);
    }
  }
}

#[cfg(feature = "egui")]
pub use egui_ctx::*;



// impl helper trait

impl ScreenDescriptorExtension for ScreenDescriptor {

  fn new(size_in_pixels: [u32; 2], pixels_per_point: f32) -> Self {
    Self { size_in_pixels, pixels_per_point }
  }

  fn from_window(window: &Window) -> Self {
    let size = window.inner_size();
    Self::new([size.width, size.height], window.scale_factor() as f32)
  }

  fn clone(&self) -> Self {
    Self::new(self.size_in_pixels, self.pixels_per_point)
  }

  fn clip_rect(&self) -> Rect {
    let sf = self.pixels_per_point;
    let [w, h] = self.size_in_pixels;
    [[0.0, 0.0].into(), [w as f32/sf, h as f32/sf].into()].into()
  }
}
