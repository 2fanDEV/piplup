use nalgebra::Matrix4;
use render_object::RenderObject;

pub mod render_object;
pub mod material;

pub struct DrawContext {
    pub opaque_surfaces: Vec<RenderObject>
}

pub trait Renderable {
    fn draw(&self, top_matrix: Matrix4<f32>, draw_ctx: &mut DrawContext);
}

pub trait RenderNode {}
