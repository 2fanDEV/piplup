use std::{ops::Deref, sync::Arc};

use ash::vk::{
    CommandBuffer, CommandBufferAllocateInfo, CommandBufferBeginInfo, CommandBufferLevel,
    CommandBufferUsageFlags, CommandPool, CommandPoolCreateFlags, CommandPoolCreateInfo, Fence,
    SubmitInfo,
};

use super::{device::VkDevice, queue::VkQueue};

#[derive(Clone)]
pub struct VkCommandPool {
    pub device: Arc<VkDevice>,
    command_pool: CommandPool,
}

impl Deref for VkCommandPool {
    type Target = CommandPool;

    fn deref(&self) -> &Self::Target {
        &self.command_pool
    }
}

impl VkCommandPool {
    pub fn new(queue: Arc<VkQueue>) -> VkCommandPool {
        let create_info = CommandPoolCreateInfo::default()
            .flags(CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue.queue_family_index);
        let command_pool = unsafe {
            queue
                .device
                .create_command_pool(&create_info, None)
                .unwrap()
        };

        Self {
            device: queue.device.clone(),
            command_pool,
        }
    }

    pub fn allocate_command_buffer(&self) -> CommandBuffer {
        let allocate_info = CommandBufferAllocateInfo::default()
            .command_pool(self.command_pool)
            .level(CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        unsafe {
            self.device
                .allocate_command_buffers(&allocate_info)
                .unwrap()[0]
        }
    }

    pub fn allocate_command_buffers(&self, count: u32) -> Vec<CommandBuffer> {
        let allocate_info = CommandBufferAllocateInfo::default()
            .command_pool(self.command_pool)
            .level(CommandBufferLevel::PRIMARY)
            .command_buffer_count(count);
        unsafe {
            self.device
                .allocate_command_buffers(&allocate_info)
                .unwrap()
        }
    }

    pub fn single_time_command(&self) -> Result<CommandBuffer, ()> {
        let command_buffer_allocate_info = CommandBufferAllocateInfo::default()
            .level(CommandBufferLevel::PRIMARY)
            .command_pool(self.command_pool)
            .command_buffer_count(1);

        let command_buffers = unsafe {
            self.device
                .allocate_command_buffers(&command_buffer_allocate_info)
                .unwrap()
        };

        let command_buffer_begin_info =
            CommandBufferBeginInfo::default().flags(CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe {
            self.device
                .begin_command_buffer(command_buffers[0], &command_buffer_begin_info)
                .unwrap()
        };

        Ok(command_buffers[0])
    }

    pub fn end_single_time_command(&self, queue: Arc<VkQueue>, command_buffer: CommandBuffer) {
        let command_buffers = vec![command_buffer];
        unsafe {
            self.device.end_command_buffer(command_buffer).unwrap();
            let submit_info = vec![SubmitInfo::default().command_buffers(&command_buffers)];
            self.device
                .queue_submit(**queue, &submit_info, Fence::null())
                .unwrap();
            self.device.queue_wait_idle(**queue).unwrap();
            self.device
                .free_command_buffers(self.command_pool, &command_buffers);
        };
    }
}
