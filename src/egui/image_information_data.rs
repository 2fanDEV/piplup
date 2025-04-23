use std::{env::Args, sync::Arc};

use ash::Device;
use egui::{epaint::ImageDelta, FullOutput, ImageData, TextureId};

use crate::components::{
    allocated_image::AllocatedImage, descriptors::DescriptorSetDetails,
    memory_allocator::MemoryAllocator,
};

#[derive(Debug)]
pub struct TextureInformationData {
    pub allocated_image: AllocatedImage,
    pub descriptor_set_details: DescriptorSetDetails,
    pub texture_id: TextureId,
}

impl TextureInformationData {
    pub fn new<T,D>(
        full_output: FullOutput,
        texture_delta_tuple: (TextureId, ImageDelta),
        image_creator: T,
        descriptor_creator: D
    ) -> Self
    where
        T: FnOnce(&ImageData) -> AllocatedImage,
        D: FnOnce(&AllocatedImage) -> DescriptorSetDetails
    {
        let allocated_image = image_creator(&texture_delta_tuple.1.image);
        let descriptor_set_details = descriptor_creator(&allocated_image);
        Self {
            allocated_image,
            descriptor_set_details,
            texture_id: texture_delta_tuple.0,
        }
    }
}
