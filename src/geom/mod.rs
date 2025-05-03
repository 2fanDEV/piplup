use ash::vk::{VertexInputAttributeDescription, VertexInputBindingDescription};
use log::debug;
use nalgebra::Matrix4;
use push_constants::PushConstant;
use winit::window::Window;

pub mod mesh;
pub mod push_constants;
pub mod vertex_2d;
pub mod vertex_3d;
pub trait VertexAttributes {
    fn get_binding_description() -> Vec<VertexInputBindingDescription>;

    fn get_attribute_description() -> Vec<VertexInputAttributeDescription>;
}

pub fn egui_push_constant(window: &Window) -> Vec<u8> {
    let scale_factor = window.scale_factor();
    let logical_size = window.inner_size().to_logical::<f32>(scale_factor);

    let sx = 2.0 / logical_size.width;
    let sy = 2.0 / logical_size.height;
    let tx = -1.0;
    let ty = -1.0;

    let push_constant = PushConstant::new(
        Matrix4::new(
            sx, 0.0, 0.0, tx, // Column 1
            0.0, sy, 0.0, ty, // Column 2
            0.0, 0.0, 1.0, 0.0, // Column 3
            0.0, 0.0, 0.0, 1.0, // Column 4
        ),
    );
    push_constant.raw_data()
}

pub fn triangle_push_constant() -> Vec<u8> {
    let push_constant = PushConstant::new(Matrix4::<f32>::identity());
    push_constant.raw_data()
}
