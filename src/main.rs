use log::{debug, LevelFilter};
use piplup::app::App;
use winit::event_loop::EventLoop;

fn main() {
    println!("Hello, world!");
    let mut app = App::default();
    let event_loop = EventLoop::new().unwrap();
    let i = 32;
    env_logger::Builder::new()
        .filter_level(LevelFilter::Debug)
        .try_init()
        .unwrap();

    debug!("START APP");
    event_loop.run_app(&mut app).unwrap();
}
