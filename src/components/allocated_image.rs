use ash::vk::{Extent3D, Format, Image, ImageView};
use vk_mem::Allocation;


#[derive(Debug)]
pub struct AllocatedImage {
    pub image: Image,
    pub image_view: ImageView,
    pub allocation: vk_mem::Allocation,
    pub extent: Extent3D,
    pub image_format: Format,
}

impl AllocatedImage {
    pub fn new(
        image: Image,
        image_view: ImageView,
        allocation: Allocation,
        extent: Extent3D,
        image_format: Format,
    ) -> Self {
        Self {
            image,
            image_view,
            extent,
            allocation,
            image_format,
        }
    }
}
