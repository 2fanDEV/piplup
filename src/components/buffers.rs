use std::{io::Error, ops::Deref, sync::Arc};

use ash::vk::{
    Buffer, BufferCopy, BufferCreateInfo, BufferUsageFlags, DeviceSize, Extent2D, Framebuffer,
    FramebufferCreateInfo, MemoryPropertyFlags, Queue, SharingMode,
};
use vk_mem::{Alloc, Allocation, AllocationCreateFlags, AllocationCreateInfo, MemoryUsage};

use super::{
    command_buffers::VkCommandPool, device::VkDevice, queue::VkQueue, render_pass::VkRenderPass,
    swapchain::ImageDetails,
};

pub struct VkFrameBuffer {
    frame_buffer: Framebuffer,
    device: Arc<VkDevice>,
}

impl Deref for VkFrameBuffer {
    type Target = Framebuffer;

    fn deref(&self) -> &Self::Target {
        &self.frame_buffer
    }
}

impl VkFrameBuffer {
    pub fn new(
        device: Arc<VkDevice>,
        render_pass: Arc<VkRenderPass>,
        extent: Extent2D,
        image_details: &ImageDetails,
    ) -> Self {
        let image_views = vec![image_details.image_view];
        let create_info = FramebufferCreateInfo::default()
            .render_pass(**render_pass)
            .width(extent.width)
            .height(extent.height)
            .layers(1)
            .attachments(&image_views);
        Self {
            device: device.clone(),
            frame_buffer: unsafe { device.create_framebuffer(&create_info, None).unwrap() },
        }
    }

    pub fn create_framebuffers(
        vk_device: Arc<VkDevice>,
        render_pass: Arc<VkRenderPass>,
        extent: Extent2D,
        image_details: &[ImageDetails],
    ) -> Vec<VkFrameBuffer> {
        image_details
            .iter()
            .map(|image_detail| {
                Self::new(vk_device.clone(), render_pass.clone(), extent, image_detail)
            })
            .collect::<Vec<VkFrameBuffer>>()
    }
}

#[derive(Debug)]
pub struct VkBuffer {
    pub buffer: Buffer,
    pub allocation: Allocation,
}

impl Deref for VkBuffer {
    type Target = Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl VkBuffer {
    pub fn create_buffer<T>(
        allocator: &vk_mem::Allocator,
        buffer_elements: &[T],
        queues: &[Arc<VkQueue>],
        buffer_usage: BufferUsageFlags,
        memory_usage: MemoryUsage,
        memory_property_flags: MemoryPropertyFlags,
        command_pool: &VkCommandPool,
    ) -> Result<VkBuffer, Error> {
        let buffer_size = buffer_elements.len() * size_of::<T>();
        let mut staging_buffer = Self::allocate_buffer(
            allocator,
            buffer_elements,
            queues,
            BufferUsageFlags::TRANSFER_SRC,
            MemoryUsage::Auto,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let data = unsafe {
            allocator
                .map_memory(&mut staging_buffer.allocation)
                .unwrap()
        };
        unsafe {
            std::ptr::copy_nonoverlapping(
                buffer_elements.as_ptr(),
                data as *mut T,
                buffer_elements.len(),
            );
            allocator.unmap_memory(&mut staging_buffer.allocation);
        };

        let buffer = Self::allocate_buffer(
            allocator,
            buffer_elements,
            queues,
            BufferUsageFlags::TRANSFER_DST | buffer_usage,
            memory_usage,
            memory_property_flags,
        )?;

        Self::copy_buffer(
            *staging_buffer,
            *buffer,
            buffer_size as u64,
            queues[0].clone(),
            command_pool,
        );

        unsafe { allocator.destroy_buffer(staging_buffer.buffer, &mut staging_buffer.allocation) };
        Ok(buffer)
    }

    fn allocate_buffer<T>(
        allocator: &vk_mem::Allocator,
        buffer_elements: &[T],
        queues: &[Arc<VkQueue>],
        buffer_usage: BufferUsageFlags,
        memory_usage: MemoryUsage,
        memory_property_flags: MemoryPropertyFlags,
    ) -> Result<VkBuffer, Error> {
        let queue_family_indices = queues
            .iter()
            .map(|queue| queue.queue_family_index)
            .collect::<Vec<u32>>();

        let buffer_size = buffer_elements.len() * size_of::<T>();
        let buffer_info = BufferCreateInfo::default()
            .size(buffer_size as u64)
            .sharing_mode(SharingMode::EXCLUSIVE)
            .queue_family_indices(&queue_family_indices) // not sure about this
            .usage(buffer_usage);
        let create_info = AllocationCreateInfo {
            usage: memory_usage,
            required_flags: memory_property_flags,
            flags: AllocationCreateFlags::MAPPED,
            ..Default::default()
        };
        let (buffer, allocation) =
            unsafe { allocator.create_buffer(&buffer_info, &create_info).unwrap() };
        unsafe { allocator.bind_buffer_memory(&allocation, buffer).unwrap() }
        Ok(Self { buffer, allocation })
    }

    fn copy_buffer(
        src: Buffer,
        dst: Buffer,
        size: DeviceSize,
        queue: Arc<VkQueue>,
        command_pool: &VkCommandPool,
    ) {
        let command_buffer = command_pool.single_time_command().unwrap();
        let buffer_copy = vec![BufferCopy::default().src_offset(0).dst_offset(0).size(size)];
        unsafe {
            command_pool
                .device
                .cmd_copy_buffer(command_buffer, src, dst, &buffer_copy)
        };
        command_pool.end_single_time_command(queue, command_buffer);
    }

    fn find_memory_type_bits(
        device: Arc<VkDevice>,
        type_filter: u32,
        flags: MemoryPropertyFlags,
    ) -> usize {
        let instance = &device.instance;
        let physical_device = device.physical_device;
        let properties = unsafe { instance.get_physical_device_memory_properties(physical_device) };
        let memory_property = properties
            .memory_types
            .into_iter()
            .enumerate()
            .filter(|(idx, mem_type)| {
                ((type_filter & (1 << idx) != 0)
                    && (properties.memory_types[*idx].property_flags & flags)
                        != MemoryPropertyFlags::empty())
            })
            .collect::<Vec<_>>();
        memory_property[0].0
    }
}
