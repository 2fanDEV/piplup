use std::{fmt::Debug, iter::Sum, marker::PhantomData};

use crate::components::allocation_types::VkBuffer;
use anyhow::Error;
use ash::vk::{BufferUsageFlags, MemoryPropertyFlags, Rect2D, Viewport};
use egui::TextureId;
use vk_mem::MemoryUsage;

use super::VertexAttributes;

#[derive(Debug)]
pub struct Mesh<T, U>
where
    T: VertexAttributes,
    U: Sum,
{
    pub vertices: Vec<T>,
    pub indices: Vec<U>,
    pub texture_id: Option<TextureId>,
    pub scissors: Rect2D,
    pub viewport: Viewport,
}

#[derive(Debug)]
pub struct MeshBuffers<T, U>
where
    T: VertexAttributes,
    U: Sum,
{
    pub vertex_buffer: VkBuffer,
    pub index_buffer: VkBuffer,
    pub mesh: Mesh<T, U>,
    pub indices: PhantomData<U>,
}

impl<T, U> MeshBuffers<T, U>
where
    T: VertexAttributes + Clone,
    U: Sum + Clone,
{
    #[allow(deprecated)]
    pub fn new(
        mesh: Mesh<T, U>,
        create_vertex_buffer: impl FnOnce(
            Vec<T>,
            BufferUsageFlags,
            MemoryUsage,
            MemoryPropertyFlags,
        ) -> VkBuffer,
        create_index_buffer: impl FnOnce(
            Vec<U>,
            BufferUsageFlags,
            MemoryUsage,
            MemoryPropertyFlags,
        ) -> VkBuffer,
    ) -> Result<MeshBuffers<T, U>, Error> {
        let vertex_buffer = create_vertex_buffer(
            mesh.vertices.clone(),
             BufferUsageFlags::VERTEX_BUFFER | BufferUsageFlags::STORAGE_BUFFER | BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            MemoryUsage::GpuOnly,
            MemoryPropertyFlags::DEVICE_LOCAL,
        );
        let index_buffer = create_index_buffer(
            mesh.indices.clone(),
            BufferUsageFlags::INDEX_BUFFER | BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            MemoryUsage::GpuOnly,
            MemoryPropertyFlags::DEVICE_LOCAL,
        );
        Ok(Self {
            vertex_buffer,
            index_buffer,
            mesh,
            indices: PhantomData,
        })
    }
}
