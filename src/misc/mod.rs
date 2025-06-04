use nalgebra::Matrix4;

pub mod render_object;
pub mod material;

pub struct DrawContext;

pub trait Renderable {
    fn draw(&self, top_matrix: Matrix4<f32>, draw_ctx: &DrawContext);
}
