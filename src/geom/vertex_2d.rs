use std::mem::offset_of;

use ash::vk::{
    Format, VertexInputAttributeDescription, VertexInputBindingDescription, VertexInputRate,
};
use nalgebra::{Vector2, Vector4};

use super::VertexAttributes;

#[derive(Debug, Clone)]
pub struct Vertex2D {
    pub pos: Vector2<f32>,
    pub texture_coords: Vector2<f32>,
    pub color: Vector4<u8>,
}

impl VertexAttributes for Vertex2D {
    fn get_binding_description() -> Vec<VertexInputBindingDescription> {
        vec![VertexInputBindingDescription::default()
            .binding(0)
            .stride(size_of::<Vertex2D>() as u32)
            .input_rate(VertexInputRate::VERTEX)]
    }

    fn get_attribute_description() -> Vec<VertexInputAttributeDescription> {
        let mut attribute_descriptons: [VertexInputAttributeDescription; 3] =
            [Default::default(); 3];
        attribute_descriptons[0] = attribute_descriptons[0]
            .binding(0)
            .location(0)
            .format(Format::R32G32_SFLOAT)
            .offset(offset_of!(Vertex2D, pos) as u32);

        attribute_descriptons[1] = attribute_descriptons[1]
            .binding(0)
            .location(1)
            .format(Format::R32G32_SFLOAT)
            .offset(offset_of!(Vertex2D, texture_coords) as u32);

        attribute_descriptons[2] = attribute_descriptons[2]
            .binding(0)
            .location(2)
            .format(Format::R8G8B8A8_SRGB)
            .offset(offset_of!(Vertex2D, color) as u32);
        attribute_descriptons.to_vec()
    }
}
