use std::sync::Arc;

use ash::{
    vk::{
        AccessFlags, CommandBuffer, DependencyFlags, Extent2D, Extent3D, Filter, Format, Image, ImageAspectFlags, ImageBlit, ImageCreateInfo, ImageLayout, ImageMemoryBarrier, ImageSubresourceLayers, ImageSubresourceRange, ImageTiling, ImageType, ImageUsageFlags, ImageViewCreateInfo, ImageViewType, Offset3D, PipelineStageFlags, SampleCountFlags, REMAINING_ARRAY_LAYERS, REMAINING_MIP_LEVELS
    },
    Device,
};

use super::device::VkDevice;

pub fn image_create_info<'a>(
    format: Format,
    flags: ImageUsageFlags,
    extent: Extent3D,
    initial_layout: Option<ImageLayout>
) -> ImageCreateInfo<'a> {
    ImageCreateInfo::default()
        .format(format)
        .extent(extent)
        .usage(flags)
        .image_type(ImageType::TYPE_2D)
        .mip_levels(1)
        .array_layers(1)
        .samples(SampleCountFlags::TYPE_1)
        .tiling(ImageTiling::OPTIMAL)
        .initial_layout(initial_layout.unwrap_or(ImageLayout::UNDEFINED))
}

pub fn image_view_create_info<'a>(
    image: Image,
    format: Format,
    aspect_flags: ImageAspectFlags,
) -> ImageViewCreateInfo<'a> {
    ImageViewCreateInfo::default()
        .format(format)
        .image(image)
        .view_type(ImageViewType::TYPE_2D)
        .subresource_range(
            image_subresource_range(aspect_flags)
                .level_count(1)
                .layer_count(1),
        )
}

pub fn image_transition(
    device: Arc<VkDevice>,
    command_buffer: CommandBuffer,
    queue_family_idx: u32,
    image: Image,
    current_image_layout: ImageLayout,
    new_image_layout: ImageLayout,
) {
    let sub_resource_range = image_subresource_range(
        if new_image_layout.eq(&ImageLayout::DEPTH_ATTACHMENT_OPTIMAL) {
            ImageAspectFlags::DEPTH
        } else {
            ImageAspectFlags::COLOR
        },
    );

    let image_memory_barrier = ImageMemoryBarrier::default()
        .src_access_mask(AccessFlags::COLOR_ATTACHMENT_READ)
        .dst_access_mask(AccessFlags::SHADER_READ)
        .old_layout(current_image_layout)
        .new_layout(new_image_layout)
        .src_queue_family_index(queue_family_idx)
        .dst_queue_family_index(queue_family_idx)
        .image(image)
        .subresource_range(sub_resource_range);

    unsafe {
        device.cmd_pipeline_barrier(
            command_buffer,
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            PipelineStageFlags::FRAGMENT_SHADER,
            DependencyFlags::empty(),
            &[],
            &[],
            &[image_memory_barrier],
        )
    };
}

pub fn image_subresource_range(aspect_flag: ImageAspectFlags) -> ImageSubresourceRange {
    ImageSubresourceRange::default()
        .aspect_mask(aspect_flag)
        .base_mip_level(0)
        .level_count(REMAINING_MIP_LEVELS)
        .base_array_layer(0)
        .layer_count(REMAINING_ARRAY_LAYERS)
}

pub fn image_subresource_layers(aspect_flag: ImageAspectFlags) -> ImageSubresourceLayers {
    ImageSubresourceLayers::default()
        .layer_count(1)
        .mip_level(0)
        .base_array_layer(0)
        .aspect_mask(aspect_flag)
}

pub fn copy_image_to_image(
    device: &Device,
    command_buffer: CommandBuffer,
    src_image: Image,
    dst_image: Image,
    src_extent: Extent2D,
    dst_extent: Extent2D,
) {
    let src_offset_3d = [
        Offset3D::default()
            .x(src_extent.width as i32)
            .y(src_extent.height as i32)
            .z(0),
        Offset3D::default()
            .x(0)
            .y(0)
            .z(1),
    ];

    let dst_offset_3d = [
           Offset3D::default()
            .x(0)
            .y(0)
            .z(0),
        Offset3D::default()
            .x(dst_extent.width as i32)
            .y(dst_extent.height as i32)
            .z(1),
    ];

    let src_image_subresource_layers = ImageSubresourceLayers::default()
        .aspect_mask(ImageAspectFlags::COLOR)
        .layer_count(1)
        .mip_level(0)
        .base_array_layer(0);

    let dst_image_subresource_layers = ImageSubresourceLayers::default()
        .aspect_mask(ImageAspectFlags::COLOR)
        .layer_count(1)
        .base_array_layer(0)
        .mip_level(0);

    let regions_blit = vec![ImageBlit::default()
        .src_offsets(src_offset_3d)
        .dst_offsets(dst_offset_3d)
        .src_subresource(src_image_subresource_layers)
        .dst_subresource(dst_image_subresource_layers)];
   
    unsafe {
        device.cmd_blit_image(
            command_buffer,
            src_image,
            ImageLayout::TRANSFER_SRC_OPTIMAL,
            dst_image,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            &regions_blit,
            Filter::LINEAR,
        );
    }
}
