use ash::vk::{VertexInputAttributeDescription, VertexInputBindingDescription};

pub mod vertex_2d;
pub mod vertex_3d;

pub trait VertexAttributes {

   fn get_binding_description() -> Vec<VertexInputBindingDescription>;

   fn get_attribute_description() -> Vec<VertexInputAttributeDescription>;

}
