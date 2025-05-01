use std::{io::Error, ops::Deref, sync::Arc};

use ash::vk::{
    BufferCreateInfo, BufferUsageFlags, Extent3D, Format, ImageAspectFlags, ImageLayout,
    ImageUsageFlags, MemoryPropertyFlags, SharingMode,
};
use egui::{Color32, ImageData};
use vk_mem::{
    Alloc, AllocationCreateFlags, AllocationCreateInfo, AllocatorCreateInfo, MemoryUsage,
};


use super::{
    allocation_types::{AllocatedImage, VkBuffer}, command_buffers::VkCommandPool, device::VkDevice, image_util::{image_create_info, image_transition, image_view_create_info}, queue::VkQueue, swapchain::KHRSwapchain
};

pub struct MemoryAllocator {
    allocator: vk_mem::Allocator,
    device: Arc<VkDevice>,
}

impl Deref for MemoryAllocator {
    type Target = vk_mem::Allocator;

    fn deref(&self) -> &Self::Target {
        &self.allocator
    }
}

impl MemoryAllocator {
    pub fn new(device: Arc<VkDevice>, allocator_create_info: AllocatorCreateInfo) -> Self {
        Self {
            allocator: unsafe { vk_mem::Allocator::new(allocator_create_info).unwrap() },
            device,
        }
    }

    #[allow(deprecated)]
    pub fn create_image(&self, swapchain: Arc<KHRSwapchain>) -> Result<AllocatedImage, Error> {
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
            None,
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

    pub fn create_texture_image(
        &self,
        queues: &[Arc<VkQueue>],
        command_pool: &VkCommandPool,
        image_data: &ImageData,
    ) -> Result<AllocatedImage, &str> {
        let pixels = match image_data {
            ImageData::Color(color_image) => color_image.pixels.clone(),
            ImageData::Font(font_image) => font_image.srgba_pixels(None).collect::<Vec<Color32>>(),
        };

        let staging_buffer = self
            .staging_buffer((size_of::<f32>() * pixels.len()) as u64, &pixels, queues)
            .unwrap();
        let format = Format::R8G8B8A8_SRGB;

        let extent = Extent3D::default()
            .height(image_data.height() as u32)
            .width(image_data.width() as u32)
            .depth(1);
        let image_info = image_create_info(
            format,
            ImageUsageFlags::TRANSFER_DST
                | ImageUsageFlags::COLOR_ATTACHMENT
                | ImageUsageFlags::SAMPLED,
            extent,
            Some(ImageLayout::UNDEFINED),
        );
        let create_info = Self::allocation_create_info(
            AllocationCreateFlags::MAPPED | AllocationCreateFlags::HOST_ACCESS_RANDOM,
            MemoryPropertyFlags::DEVICE_LOCAL,
            None,
            MemoryUsage::Auto,
            None,
        );
        let (image, allocation) = unsafe {
            self.allocator
                .create_image(&image_info, &create_info)
                .unwrap()
        };

        let single_time_command = command_pool.single_time_command().unwrap();
        image_transition(
            command_pool.device.clone(),
            single_time_command,
            queues[0].queue_family_index,
            image,
            ImageLayout::UNDEFINED,
            ImageLayout::TRANSFER_DST_OPTIMAL,
        );
        command_pool.end_single_time_command(queues[0].clone(), single_time_command);

        VkBuffer::copy_buffer_to_image(
            *staging_buffer,
            image,
            extent,
            queues[0].clone(),
            command_pool,
        )
        .unwrap();

        let single_time_command = command_pool.single_time_command().unwrap();
        image_transition(
            command_pool.device.clone(),
            single_time_command,
            queues[0].queue_family_index,
            image,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        command_pool.end_single_time_command(queues[0].clone(), single_time_command);
        let image_view_create_info = image_view_create_info(image, format, ImageAspectFlags::COLOR);
        let image_view = unsafe {
            self.device
                .create_image_view(&image_view_create_info, None)
                .unwrap()
        };
        Ok(AllocatedImage {
            image,
            image_view,
            allocation,
            extent,
            image_format: format,
        })
    }

    fn staging_buffer<T>(
        &self,
        buffer_size: u64,
        buffer_elements: &[T],
        queues: &[Arc<VkQueue>],
    ) -> Result<VkBuffer, Error>
    where
        T: Clone,
    {
        let mut staging_buffer = self.allocate_single_buffer(
            buffer_size,
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
        Ok(staging_buffer)
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
        T:Clone,
    {       
        let buffer_size = std::mem::size_of_val(buffer_elements) as u64;
        let mut staging_buffer = self.staging_buffer(buffer_size, buffer_elements, queues)?;
        let data = unsafe { self.map_memory(&mut staging_buffer.allocation).unwrap() };
        unsafe {
            std::ptr::copy_nonoverlapping(
                buffer_elements.as_ptr(),
                data as *mut T,
                buffer_elements.len(),
            );
            self.unmap_memory(&mut staging_buffer.allocation);
        };

        let buffer = self.allocate_single_buffer(
            buffer_size,
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

    pub fn allocate_single_buffer(
        &self,
        buffer_size: u64,
        queues: &[Arc<VkQueue>],
        buffer_usage: BufferUsageFlags,
        memory_usage: MemoryUsage,
        memory_property_flags: MemoryPropertyFlags,
    ) -> Result<VkBuffer, Error> {
        let queue_family_indices = queues
            .iter()
            .map(|queue| queue.queue_family_index)
            .collect::<Vec<u32>>();

        let buffer_info = BufferCreateInfo::default()
            .size(buffer_size)
            .sharing_mode(SharingMode::EXCLUSIVE)
            .queue_family_indices(&queue_family_indices) // not sure about this
            .usage(buffer_usage);
        let create_info = AllocationCreateInfo {
            usage: memory_usage,
            required_flags: memory_property_flags,
            flags: AllocationCreateFlags::MAPPED,
            ..Default::default()
        };
        let (buffer, allocation) = unsafe {
            self.allocator
                .create_buffer(&buffer_info, &create_info)
                .unwrap()
        };
        //       unsafe { allocator.bind_buffer_memory(&allocation, buffer).unwrap() }
        Ok(VkBuffer { buffer, allocation })
    }

    fn allocation_create_info(
        flags: AllocationCreateFlags,
        required_flags: MemoryPropertyFlags,
        preferred_flags: Option<MemoryPropertyFlags>,
        memory_usage: MemoryUsage,
        memory_type_bits: Option<u32>,
    ) -> AllocationCreateInfo {
        let mut allocation_create_info = AllocationCreateInfo::default();
        allocation_create_info.flags = flags;
        allocation_create_info.required_flags = required_flags;
        allocation_create_info.preferred_flags =
            preferred_flags.unwrap_or(MemoryPropertyFlags::empty());
        allocation_create_info.memory_type_bits = memory_type_bits.unwrap_or(0);
        allocation_create_info.usage = memory_usage;
        allocation_create_info
    }
}
