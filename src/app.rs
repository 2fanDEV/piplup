use winit::{application::ApplicationHandler, window::{Window, WindowAttributes}};

#[derive(Default)]
pub struct App {
    window: Option<Window>
}


impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = WindowAttributes::default();
        self.window = event_loop.create_window(window_attributes).ok();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
    }
}
