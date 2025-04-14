use std::sync::Arc;

use ash::vk::{
    CommandBuffer, CommandBufferAllocateInfo, CommandBufferLevel, CommandPool, CommandPoolCreateFlags, CommandPoolCreateInfo, Fence, FenceCreateFlags, FenceCreateInfo, Semaphore, SemaphoreCreateFlags, SemaphoreCreateInfo
};

use super::{command_buffers::{self, VkCommandPool}, device::VkDevice, queue::VkQueue};

#[derive(Clone)]
pub struct FrameData {
    pub command_pool: VkCommandPool,
    pub command_buffer: CommandBuffer,
    pub imm_command_buffer: CommandBuffer,
    pub render_semaphore: Vec<Semaphore>,
    pub swapchain_semaphore: Vec<Semaphore>,
    pub render_fence: Vec<Fence>,
}

impl FrameData {
    pub fn new(device: Arc<VkDevice>, queue: Arc<VkQueue>) -> Self {
        unsafe {
        let command_pool = VkCommandPool::new(queue);
            Self {
                command_pool: command_pool.clone(),
                command_buffer: device.allocate_command_buffers(&allocate_command_buffer_info(*command_pool)).unwrap()[0],
                imm_command_buffer: device.allocate_command_buffers(&allocate_command_buffer_info(*command_pool)).unwrap()[0],
                render_semaphore: vec![device.create_semaphore(&create_semaphore_info(), None).unwrap()],
                swapchain_semaphore: vec![device.create_semaphore(&create_semaphore_info(), None).unwrap()],
                render_fence: vec![device.create_fence(&create_fence_info(), None).unwrap()],
            }
        }
    }

    pub fn get_command_buffer(&self) -> Vec<CommandBuffer> {
        vec![self.command_buffer]
    }
}

pub fn create_command_pool_info(queue_family_index: u32, flags: CommandPoolCreateFlags) -> CommandPoolCreateInfo<'static> {
    CommandPoolCreateInfo::default().flags(flags)
        .queue_family_index(queue_family_index)
}

pub fn allocate_command_buffer_info(command_pool: CommandPool) -> CommandBufferAllocateInfo<'static> {
    CommandBufferAllocateInfo::default()
        .level(CommandBufferLevel::PRIMARY)
        .command_buffer_count(1)
        .command_pool(command_pool)
}

pub fn create_semaphore_info() -> SemaphoreCreateInfo<'static> {
    SemaphoreCreateInfo::default().flags(SemaphoreCreateFlags::empty())
}

pub fn create_fence_info() -> FenceCreateInfo<'static> {
    FenceCreateInfo::default().flags(FenceCreateFlags::SIGNALED)
}
