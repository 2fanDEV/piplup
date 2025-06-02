pub mod render_object;
pub mod material;


pub trait Renderable {
    fn draw(&self);
}
