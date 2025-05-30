use std::{any::Any, ffi, fmt::Debug, io::Error, ops::Deref, sync::Arc};

use ash::vk::{
    BufferCreateInfo, BufferDeviceAddressInfo, BufferUsageFlags, Extent3D, Format,
    ImageAspectFlags, ImageLayout, ImageUsageFlags, MemoryPropertyFlags, Packed24_8, SharingMode,
};
use egui::{Color32, ImageData};
use vk_mem::{
    Alloc, Allocation, AllocationCreateFlags, AllocationCreateInfo, AllocatorCreateInfo,
    MemoryUsage,
};

use super::{
    allocation_types::{AllocatedImage, VkBuffer},
    command_buffers::{self, VkCommandPool},
    device::VkDevice,
    image_util::{image_create_info, image_transition, image_view_create_info},
    queue::VkQueue,
    swapchain::{ImageDetails, KHRSwapchain},
};

#[derive(Debug, Clone, Copy)]
pub enum AllocationUnitType {
    Buffer(VkBuffer),
    Image(AllocatedImage),
}

#[derive(Debug)]
pub struct AllocationUnit {
    pub unit: AllocationUnitType,
    pub allocation: Allocation,
}

impl AllocationUnitType {
    pub fn get_cloned<T: Any + Clone>(&self) -> T {
        match self {
            AllocationUnitType::Buffer(buffer) => (buffer as &dyn Any).downcast_ref::<T>().cloned(),
            AllocationUnitType::Image(image) => (image as &dyn Any).downcast_ref::<T>().cloned(),
        }
        .unwrap()
    }

    pub fn get_copied<T: Any + Copy>(&self) -> T {
        match self {
            AllocationUnitType::Buffer(buffer) => (buffer as &dyn Any).downcast_ref::<T>().copied(),
            AllocationUnitType::Image(image) => (image as &dyn Any).downcast_ref::<T>().copied(),
        }
        .unwrap()
    }
}

pub struct MemoryAllocator {
    allocator: vk_mem::Allocator,
    device: Arc<VkDevice>,
    queues: Vec<Arc<VkQueue>>,
}

impl Deref for MemoryAllocator {
    type Target = vk_mem::Allocator;

    fn deref(&self) -> &Self::Target {
        &self.allocator
    }
}

impl MemoryAllocator {
    pub fn new(
        device: Arc<VkDevice>,
        queues: &[Arc<VkQueue>],
        allocator_create_info: AllocatorCreateInfo,
    ) -> Self {
        Self {
            device: device.clone(),
            allocator: unsafe { vk_mem::Allocator::new(allocator_create_info).unwrap() },
            queues: queues.to_vec(),
        }
    }

    #[allow(deprecated)]
    pub fn create_image(
        &self,
        extent: Extent3D,
        swapchain: Arc<KHRSwapchain>,
        format: Format,
        initial_layout: Option<ImageLayout>,
        flags: ImageUsageFlags,
        aspect_flags: ImageAspectFlags,
        mipmapped: bool,
    ) -> Result<AllocationUnit, Error> {
        let image_create_info = image_create_info(
            format,
            ImageUsageFlags::TRANSFER_SRC | ImageUsageFlags::TRANSFER_DST | flags,
            extent,
            initial_layout,
            mipmapped,
        );

        let mut allocation_create_info = AllocationCreateInfo::default();
        allocation_create_info.required_flags = MemoryPropertyFlags::DEVICE_LOCAL;
        allocation_create_info.usage = MemoryUsage::GpuOnly;

        let (image, allocation) = unsafe {
            self.allocator
                .create_image(&image_create_info, &allocation_create_info)
                .unwrap()
        };

        let image_view_create_info = image_view_create_info(image, format, aspect_flags);
        let image_view = unsafe {
            swapchain
                .device
                .create_image_view(&image_view_create_info, None)
                .unwrap()
        };
        let allocated_image =
            AllocatedImage::new(ImageDetails { image, image_view }, extent, format);
        let allocation_unit = AllocationUnit {
            unit: AllocationUnitType::Image(allocated_image),
            allocation,
        };
        Ok(allocation_unit)
    }

    pub fn create_image_with_data(
        &self,
        data: u32,
        extent: Extent3D,
        swapchain: Arc<KHRSwapchain>,
        format: Format,
        usage: ImageUsageFlags,
        aspect_flags: ImageAspectFlags,
        command_pool: &VkCommandPool,
        mipmapped: bool,
    ) -> Result<AllocationUnit, Error> {
        let data_size: u64 = (extent.depth * extent.width * extent.height * 4) as u64;
        let staging_buffer_unit = self.staging_buffer(data_size, &[data], &self.queues)?;
        let staging_buffer = staging_buffer_unit.unit.get_copied::<VkBuffer>();
        let image_unit = self.create_image(
            extent,
            swapchain,
            format,
            None,
            usage | ImageUsageFlags::TRANSFER_SRC | ImageUsageFlags::TRANSFER_DST,
            aspect_flags,
            mipmapped,
        )?;
        let image = image_unit.unit.get_copied::<AllocatedImage>();

        let cmd_buffer = command_pool.single_time_command().unwrap();
        image_transition(
            self.device.clone(),
            cmd_buffer,
            self.queues[0].queue_family_index,
            image.image_details.image,
            ImageLayout::UNDEFINED,
            ImageLayout::TRANSFER_DST_OPTIMAL,
        );
        VkBuffer::copy_buffer_to_image(*staging_buffer, image.image_details.image, extent, self.queues[0].clone(), command_pool).unwrap();
        image_transition(
            self.device.clone(),
            cmd_buffer,
            self.queues[0].queue_family_index,
            image.image_details.image,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        command_pool.end_single_time_command(self.queues[0].clone(), cmd_buffer);

        Ok(AllocationUnit {
            unit: image_unit.unit,
            allocation: image_unit.allocation,
        })
    }

    //egui only
    pub fn create_egui_texture_image(
        &self,
        command_pool: &VkCommandPool,
        image_data: &ImageData,
        mipmapped: bool,
    ) -> Result<AllocationUnit, &str> {
        let pixels = match image_data {
            ImageData::Color(color_image) => color_image.pixels.clone(),
            ImageData::Font(font_image) => font_image.srgba_pixels(None).collect::<Vec<Color32>>(),
        };

        let staging_buffer = self
            .staging_buffer(
                (size_of::<f32>() * pixels.len()) as u64,
                &pixels,
                &self.queues,
            )
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
            mipmapped,
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
            self.queues[0].queue_family_index,
            image,
            ImageLayout::UNDEFINED,
            ImageLayout::TRANSFER_DST_OPTIMAL,
        );
        command_pool.end_single_time_command(self.queues[0].clone(), single_time_command);

        VkBuffer::copy_buffer_to_image(
            *staging_buffer.unit.get_copied::<VkBuffer>(),
            image,
            extent,
            self.queues[0].clone(),
            command_pool,
        )
        .unwrap();

        let single_time_command = command_pool.single_time_command().unwrap();
        image_transition(
            command_pool.device.clone(),
            single_time_command,
            self.queues[0].queue_family_index,
            image,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        command_pool.end_single_time_command(self.queues[0].clone(), single_time_command);
        let image_view_create_info = image_view_create_info(image, format, ImageAspectFlags::COLOR);
        let image_view = unsafe {
            self.device
                .create_image_view(&image_view_create_info, None)
                .unwrap()
        };
        Ok(AllocationUnit {
            unit: AllocationUnitType::Image(AllocatedImage {
                image_details: ImageDetails { image, image_view },
                extent,
                image_format: format,
            }),
            allocation,
        })
    }

    fn staging_buffer<T>(
        &self,
        buffer_size: u64,
        buffer_elements: &[T],
        queues: &[Arc<VkQueue>],
    ) -> Result<AllocationUnit, Error>
    where
        T: Clone,
    {
        let mut staging_buffer = self.allocate_single_buffer(
            buffer_size,
            queues,
            BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::SHADER_DEVICE_ADDRESS,
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

    pub fn create_buffer_with_mapped_memory<T>(
        &self,
        buffer_elements: &[T],
        queues: &[Arc<VkQueue>],
        buffer_usage: BufferUsageFlags,
        memory_usage: MemoryUsage,
        memory_property_flags: MemoryPropertyFlags,
        command_pool: &VkCommandPool,
    ) -> Result<AllocationUnit, anyhow::Error>
    where
        T: Clone + Debug,
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
            staging_buffer.unit.get_copied::<VkBuffer>(),
            buffer.unit.get_copied::<VkBuffer>(),
            buffer_size as u64,
            queues[0].clone(),
            command_pool,
        );

        unsafe {
            self.destroy_buffer(
                *staging_buffer.unit.get_copied::<VkBuffer>(),
                &mut staging_buffer.allocation,
            )
        };
        Ok(buffer)
    }

    pub fn allocate_single_buffer(
        &self,
        buffer_size: u64,
        queues: &[Arc<VkQueue>],
        buffer_usage: BufferUsageFlags,
        memory_usage: MemoryUsage,
        memory_property_flags: MemoryPropertyFlags,
    ) -> Result<AllocationUnit, Error> {
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
        let info = BufferDeviceAddressInfo::default().buffer(buffer);
        let address = unsafe { self.device.get_buffer_device_address(&info) };
        Ok(AllocationUnit {
            unit: AllocationUnitType::Buffer(VkBuffer { buffer, address }),
            allocation,
        })
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
