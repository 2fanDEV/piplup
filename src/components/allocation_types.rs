use std::{io::Error, ops::Deref, sync::Arc};

use ash::vk::{
    BufferCopy, BufferImageCopy, DeviceSize, Extent2D, Extent3D, Format, Framebuffer, FramebufferCreateInfo, Image, ImageAspectFlags, ImageLayout, ImageView, MemoryPropertyFlags, Offset3D
};
use ash::vk::Buffer;
use vk_mem::Allocation;

use super::{
    command_buffers::VkCommandPool, device::VkDevice, image_util::image_subresource_layers, queue::VkQueue, render_pass::VkRenderPass, swapchain::ImageDetails
};


#[derive(Debug)]
pub struct AllocatedImage {
    pub image_details: ImageDetails,
    pub extent: Extent3D,
    pub image_format: Format,
}

impl AllocatedImage {
    pub fn new(
        image_details: ImageDetails,
        extent: Extent3D,
        image_format: Format,
    ) -> Self {
        Self {
            image_details,
            extent,
            image_format,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum IDENTIFIER {
    SWAPCHAIN,
    DRAW, 
    DEPTH,
}

#[allow(dead_code)]
pub struct VkFrameBuffer {
    pub identifier: IDENTIFIER,
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
    fn new(
        identifier: IDENTIFIER,
        device: Arc<VkDevice>,
        render_pass: Arc<VkRenderPass>,
        extent: Extent2D,
        image_details: &[ImageDetails],
    ) -> Self {
        let image_views = image_details.iter().map(|detail| detail.image_view).collect::<Vec<ImageView>>();
        let create_info = FramebufferCreateInfo::default()
            .render_pass(**render_pass)
            .width(extent.width)
            .height(extent.height)
            .layers(1)
            .attachments(&image_views);
        Self {
            identifier,
            device: device.clone(),
            frame_buffer: unsafe { device.create_framebuffer(&create_info, None).unwrap() },
        }
    }

    pub fn create_framebuffers(
        identifier: IDENTIFIER,
        vk_device: Arc<VkDevice>,
        render_pass: Arc<VkRenderPass>,
        extent: Extent2D,
        image_details: &[ImageDetails],
    ) -> Vec<VkFrameBuffer> {
        image_details
            .iter()
            .map(|img| -> VkFrameBuffer {
                Self::new(identifier, vk_device.clone(), render_pass.clone(), extent,&[*img])
            })
            .collect::<Vec<VkFrameBuffer>>()
    }

    pub fn create_framebuffer(
        identifier: IDENTIFIER,
        vk_device: Arc<VkDevice>,
        render_pass: Arc<VkRenderPass>,
        extent: Extent2D,
        image_details: &[ImageDetails],
    ) -> VkFrameBuffer { 
        Self::new(identifier, vk_device.clone(), render_pass.clone(), extent, image_details)
    }
}

#[derive(Debug)]
pub struct VkBuffer {
    pub buffer: Buffer,
    pub address: u64
}

impl Deref for VkBuffer {
    type Target = Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl VkBuffer {
    pub fn copy_buffer(
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

    pub fn copy_buffer_to_image(
        src: Buffer,
        dst: Image,
        extent: Extent3D,
        queue: Arc<VkQueue>,
        command_pool: &VkCommandPool,
    ) -> Result<(), Error> {
        let command_buffer = command_pool.single_time_command().unwrap();
        let image_subresource= image_subresource_layers(ImageAspectFlags::COLOR);
        let buffer_image_copy = BufferImageCopy::default()
            .buffer_offset(0)
            .image_offset(Offset3D::default().x(0).y(0).z(0))
            .image_subresource(image_subresource)
            .image_extent(extent)
            .buffer_row_length(0)
            .buffer_image_height(0);

        unsafe {
            command_pool.device.cmd_copy_buffer_to_image(
                command_buffer,
                src,
                dst,
                ImageLayout::TRANSFER_DST_OPTIMAL,
                &[buffer_image_copy],
            )
        };

        command_pool.end_single_time_command(queue, command_buffer);

        Ok(())
    }

    #[allow(dead_code, warnings)]
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
