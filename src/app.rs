use muda::dpi::LogicalSize;
use winit::{
    application::ApplicationHandler,
    window::{Window, WindowAttributes},
};

use crate::renderer::Renderer;

#[derive(Default)]
pub struct App {
    window: Option<Window>,
    renderer: Option<Renderer>,
}

#[allow(warnings)]
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = WindowAttributes::default().with_inner_size(LogicalSize::new(1920, 1080));
        self.window = event_loop.create_window(window_attributes).ok();
        self.renderer = Renderer::init(&self.window.as_ref().unwrap()).ok();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        self.renderer.as_mut().unwrap().integration.input(self.window.as_mut().unwrap(), &event);
        self.renderer
            .as_mut()
            .unwrap()
            .display(self.window.as_mut().unwrap());
    }
}
