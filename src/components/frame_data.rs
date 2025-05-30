use std::{cell::RefCell, sync::Arc};

use ash::vk::{
    CommandBuffer, CommandBufferAllocateInfo, CommandBufferLevel, CommandPool, DescriptorType,
    Fence, FenceCreateFlags, FenceCreateInfo, Semaphore, SemaphoreCreateFlags, SemaphoreCreateInfo,
};

use super::{
    command_buffers::VkCommandPool,
    deletion_queue::{DeletionQueue, DestroyCommandPoolTask, DestroyDescriptorPools, FType},
    descriptors::{DescriptorAllocator, PoolSizeRatio},
    device::VkDevice,
    memory_allocator::MemoryAllocator, queue::VkQueue,
};

pub struct FrameResources {
    pub descriptor_allocator: RefCell<DescriptorAllocator>,
    pub deletion_queue: DeletionQueue
}

pub struct FrameData {
    pub command_buffer: CommandBuffer,
    pub egui_command_buffer: CommandBuffer,
    pub render_semaphore: Vec<Semaphore>,
    pub swapchain_semaphore: Vec<Semaphore>,
    pub render_fence: Vec<Fence>,
    pub frame_resources: FrameResources
}

impl FrameResources {
    pub fn enqueue_destroy_pools(&mut self) {
      self.deletion_queue.enqueue(FType::TASK(Box::new(DestroyDescriptorPools {
             allocator: self.descriptor_allocator.clone()
         })));
    }
}

impl FrameData {
    pub fn new(
        device: Arc<VkDevice>,
        memory_allocator: Arc<MemoryAllocator>,
        queue: Arc<VkQueue>
    ) -> Self {
        let mut deletion_queue = DeletionQueue::new(device.clone(), memory_allocator.clone());
        let command_pool = VkCommandPool::new(queue);
        let descriptor_allocator = RefCell::new(DescriptorAllocator::new(
                    device.clone(),
                    16,
                    vec![PoolSizeRatio::new(
                        DescriptorType::UNIFORM_BUFFER,
                        1.0,
                    )],
                ));
         deletion_queue.enqueue(FType::TASK(Box::new(DestroyDescriptorPools {
             allocator: descriptor_allocator.clone()
         })));
        unsafe {
            Self {
                command_buffer: device
                    .allocate_command_buffers(&allocate_command_buffer_info(*command_pool))
                    .unwrap()[0],
                egui_command_buffer: device
                    .allocate_command_buffers(&allocate_command_buffer_info(*command_pool))
                    .unwrap()[0],
                render_semaphore: vec![device
                    .create_semaphore(&create_semaphore_info(), None)
                    .unwrap()],
                swapchain_semaphore: vec![device
                    .create_semaphore(&create_semaphore_info(), None)
                    .unwrap()],
                render_fence: vec![device.create_fence(&create_fence_info(), None).unwrap()],
                frame_resources: FrameResources {
                    descriptor_allocator: descriptor_allocator.clone(),
                    deletion_queue
                }
            }
        }
    }
}

pub fn allocate_command_buffer_info(
    command_pool: CommandPool,
) -> CommandBufferAllocateInfo<'static> {
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
