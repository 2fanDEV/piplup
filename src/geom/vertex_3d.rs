use std::mem::offset_of;

use ash::vk::{
    Format, VertexInputAttributeDescription, VertexInputBindingDescription, VertexInputRate,
};
use cgmath::{Vector2, Vector3, Vector4};

use super::VertexAttributes;

pub struct Vertex3D {
    pos: Vector3<f32>,
    color: Vector4<f32>,
    normal: Vector3<f32>,
    uv: Vector2<f32>,
}

impl VertexAttributes for Vertex3D {
    fn get_binding_description() -> Vec<ash::vk::VertexInputBindingDescription> {
        vec![VertexInputBindingDescription::default()
            .binding(0)
            .stride(size_of::<Vertex3D>() as u32)
            .input_rate(VertexInputRate::VERTEX)]
    }

    fn get_attribute_description() -> Vec<ash::vk::VertexInputAttributeDescription> {
        vec![
            VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex3D, pos) as u32),
            VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(Format::R32G32B32A32_SFLOAT)
                .offset(offset_of!(Vertex3D, color) as u32),
            VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex3D, normal) as u32),
            VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(Format::R32G32_SFLOAT)
                .offset(offset_of!(Vertex3D, uv) as u32),
        ]
    }
}
