
use winit::{window::Window, event::WindowEvent, dpi::PhysicalSize};
use crate::{DetectChanges, BoolFlip};
use crate::android_util::*;

#[derive(Debug, Clone)]
pub struct AndroidState {
  pub app: AndroidApp,
  pub suspended: bool,
  pub window_size: PhysicalSize<u32>,
  pub content_rect: Rect,
  pub scale_factor: f64,
  pub fullscreen: bool,
  pub fullscreen_changed: bool,
}

impl AndroidState {

  pub fn new(app: AndroidApp, window: &Window) -> Self {
    Self {
      app: app.clone(),
      suspended: false,
      window_size: window.inner_size(),
      content_rect: app.content_rect(),
      scale_factor: window.scale_factor(),
      fullscreen: false,
      fullscreen_changed: false,
    }
  }

  pub fn window_changed(&mut self, window: &Window) -> (bool, bool) {
    (
      self.window_size.note_change(&window.inner_size()),
      self.scale_factor.note_change(&window.scale_factor()),
    )
  }

  pub fn content_rect_changed(&mut self) -> bool {
    self.content_rect.note_change(&self.app.content_rect())
  }

  pub fn fullscreen_changed(&mut self) -> bool {
    self.fullscreen_changed.take()
  }

  pub fn state_changed(&mut self) -> bool {
    let fullscreen = self.fullscreen_changed();
    let content_rect = self.content_rect_changed();
    fullscreen || content_rect
  }

  pub fn inter_window_event(&mut self, event: &WindowEvent) -> bool {
    match event {
      WindowEvent::RedrawRequested => self.suspended,
      WindowEvent::Resized(size) => {
        if self.suspended { return true }
        !(self.window_size.note_change(size) || self.state_changed())
      },
      WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
        if self.suspended { return true }
        !(self.scale_factor.note_change(scale_factor) || self.state_changed())
      },
      _ => false,
    }
  }

  pub fn enable_fullscreen(&mut self) {
    enable_fullscreen(&self.app);
    self.fullscreen = true;
    self.fullscreen_changed = true;
  }

  pub fn disable_fullscreen(&mut self) {
    disable_fullscreen(&self.app);
    self.fullscreen = false;
    self.fullscreen_changed = true;
  }

}