use std::sync::Arc;

use anyhow::Error;
use ash::vk::{BufferUsageFlags, MemoryPropertyFlags, Rect2D, Viewport};
use egui::TextureId;
use crate::components::{allocation_types::VkBuffer, command_buffers::VkCommandPool, memory_allocator::MemoryAllocator, queue::VkQueue};

use super::VertexAttributes;

#[derive(Debug)]
pub struct Mesh<T> 
where T: VertexAttributes
{
    pub vertices: Vec<T>,
    pub indices: Vec<u32>,
    pub texture_id: Option<TextureId>,
    pub scissors: Rect2D,
    pub viewport: Viewport,
}

#[derive(Debug)]
pub struct MeshBuffers<T> 
where T: VertexAttributes {
    pub vertex_buffer: VkBuffer,
    pub indices_buffer: VkBuffer,
    pub mesh: Mesh<T>
}

impl <T> MeshBuffers <T> 
where T: VertexAttributes + Clone {
    #[allow(deprecated)]
    pub fn new(
        mesh: Mesh<T>,
        allocator: &MemoryAllocator,
        queue: Arc<VkQueue>,
        command_pool: &VkCommandPool,
    ) -> Result<MeshBuffers<T>, Error> {
        let queue = vec![queue];
        let vertex_buffer = allocator.create_buffer(
            &mesh.vertices,
            &queue,
            BufferUsageFlags::VERTEX_BUFFER,
            vk_mem::MemoryUsage::GpuOnly,
            MemoryPropertyFlags::DEVICE_LOCAL,
            command_pool,
        )?;
        let indices_buffer = allocator.create_buffer(
            &mesh.indices,
            &queue,
            BufferUsageFlags::INDEX_BUFFER,
            vk_mem::MemoryUsage::GpuOnly,
            MemoryPropertyFlags::DEVICE_LOCAL,
            command_pool,
        )?;
        Ok(Self {
            vertex_buffer,
            indices_buffer,
            mesh,
        })
    }
}

