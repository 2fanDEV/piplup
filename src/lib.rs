use ash::vk::{VertexInputAttributeDescription, VertexInputBindingDescription};

pub mod app;
pub mod components;
pub mod renderer;
pub mod egui;


trait VertexAttributes {

   fn get_binding_description() -> Vec<VertexInputBindingDescription>;

   fn get_attribute_description() -> Vec<VertexInputAttributeDescription>;

}
