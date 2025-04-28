
use anyhow::Error;
use egui::{epaint::ImageDelta, ImageData, TextureId};

use crate::components::{
    allocated_image::AllocatedImage, descriptors::DescriptorSetDetails,
};

#[derive(Debug)]
pub struct TextureInformationData {
    pub allocated_image: AllocatedImage,
    pub descriptor_set_details: DescriptorSetDetails,
    pub texture_id: TextureId,
}

impl TextureInformationData {
    pub fn new<T,D>(
        texture_delta_tuple: (TextureId, ImageDelta),
        image_creator: T,
        descriptor_creator: D
    ) -> Self
    where
        T: FnOnce(&ImageData) -> AllocatedImage,
        D: FnOnce(&AllocatedImage) -> Result<DescriptorSetDetails, Error>
    {
        let allocated_image = image_creator(&texture_delta_tuple.1.image);
        let descriptor_set_details = descriptor_creator(&allocated_image).unwrap();
        Self {
            allocated_image,
            descriptor_set_details,
            texture_id: texture_delta_tuple.0,
        }
    }
}
