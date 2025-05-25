use ash::vk::{DeviceAddress, Extent2D, VertexInputAttributeDescription, VertexInputBindingDescription};
use nalgebra::{Matrix4, Perspective3, Vector3};
use push_constants::PushConstant;
use winit::window::Window;

pub mod mesh;
pub mod push_constants;
pub mod vertex_2d;
pub mod vertex_3d;
pub mod assets;
pub mod scene; 

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
        u64::default(),
    );
    push_constant.raw_data_of_T()
}

pub fn triangle_push_constant(buffer_address: DeviceAddress, extent: Extent2D) -> Vec<u8> {
    let (height, width) = (extent.height, extent.width);
    let view = Matrix4::<f32>::new_translation(&Vector3::new(0.0, 0.0, -2.0));
        let proj = Perspective3::new(90.0_f32.to_radians(), width as f32/height as f32,  0.1, 1000.0).to_homogeneous();
    let wm = proj * view;
    let push_constant = PushConstant::new(wm, buffer_address);
    push_constant.raw_data()
}
