use std::{io::Error, ops::Deref, sync::Arc};

use ash::vk::{
    BufferCreateInfo, BufferUsageFlags, Extent3D, Format, ImageAspectFlags, ImageUsageFlags,
    MemoryPropertyFlags, SharingMode,
};
use vk_mem::{Alloc, AllocationCreateFlags, AllocationCreateInfo, MemoryUsage};

use super::{
    allocated_image::AllocatedImage, buffers::VkBuffer, command_buffers::VkCommandPool, image_util::{image_create_info, image_view_create_info}, queue::VkQueue, swapchain::KHRSwapchain
};

pub struct MemoryAllocator {
    allocator: vk_mem::Allocator,
}

impl Deref for MemoryAllocator {
    type Target = vk_mem::Allocator;

    fn deref(&self) -> &Self::Target {
        &self.allocator
    }
}

impl MemoryAllocator {
    #[allow(deprecated)]
    fn create_image(&self, swapchain: KHRSwapchain) -> Result<AllocatedImage, Error> {
        let extent = Extent3D::default()
            .width(swapchain.details.window_sizes.width)
            .height(swapchain.details.window_sizes.height)
            .depth(1);

        let image_create_info = image_create_info(
            Format::R16G16B16A16_SFLOAT,
            ImageUsageFlags::TRANSFER_SRC
                | ImageUsageFlags::TRANSFER_DST
                | ImageUsageFlags::STORAGE
                | ImageUsageFlags::COLOR_ATTACHMENT
                | ImageUsageFlags::SAMPLED,
            extent,
        );

        let mut allocation_create_info = AllocationCreateInfo::default();
        allocation_create_info.required_flags = MemoryPropertyFlags::DEVICE_LOCAL;
        allocation_create_info.usage = MemoryUsage::GpuOnly;

        let (image, allocation) = unsafe {
            self.allocator
                .create_image(&image_create_info, &allocation_create_info)
                .unwrap()
        };

        let image_view_create_info =
            image_view_create_info(image, Format::R16G16B16A16_SFLOAT, ImageAspectFlags::COLOR);
        let image_view = unsafe {
            swapchain
                .device
                .create_image_view(&image_view_create_info, None)
                .unwrap()
        };
        let allocated_image = AllocatedImage::new(
            image,
            image_view,
            allocation,
            extent,
            Format::R16G16B16A16_SFLOAT,
        );
        Ok(allocated_image)
    }

    pub fn create_buffer<T>(
        &self,
        buffer_elements: &[T],
        queues: &[Arc<VkQueue>],
        buffer_usage: BufferUsageFlags,
        memory_usage: MemoryUsage,
        memory_property_flags: MemoryPropertyFlags,
        command_pool: &VkCommandPool,
    ) -> Result<VkBuffer, Error>
    where
        T: Clone,
    {
        let buffer_size = buffer_elements.len() * size_of::<T>();
        let mut staging_buffer = self.allocate_buffer(
            buffer_elements,
            queues,
            BufferUsageFlags::TRANSFER_SRC,
            MemoryUsage::Unknown,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let data = unsafe { self.map_memory(&mut staging_buffer.allocation).unwrap() };
        unsafe {
            std::ptr::copy_nonoverlapping(
                buffer_elements.as_ptr(),
                data as *mut T,
                buffer_elements.len(),
            );
            self.unmap_memory(&mut staging_buffer.allocation);
        };

        let buffer = self.allocate_buffer(
            buffer_elements,
            queues,
            BufferUsageFlags::TRANSFER_DST | buffer_usage,
            memory_usage,
            memory_property_flags,
        )?;

        VkBuffer::copy_buffer(
            *staging_buffer,
            *buffer,
            buffer_size as u64,
            queues[0].clone(),
            command_pool,
        );

        unsafe { self.destroy_buffer(staging_buffer.buffer, &mut staging_buffer.allocation) };
        Ok(buffer)
    }

    fn allocate_buffer<T>(
        &self,
        buffer_elements: &[T],
        queues: &[Arc<VkQueue>],
        buffer_usage: BufferUsageFlags,
        memory_usage: MemoryUsage,
        memory_property_flags: MemoryPropertyFlags,
    ) -> Result<VkBuffer, Error>
    where
        T: Clone,
    {
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
            unsafe { self.allocator.create_buffer(&buffer_info, &create_info).unwrap() };
        //       unsafe { allocator.bind_buffer_memory(&allocation, buffer).unwrap() }
        Ok(VkBuffer { buffer, allocation })
    }
}
