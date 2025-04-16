use std::{io::Error, ops::Deref, sync::Arc};

use ash::{
    khr::swapchain,
    vk::{
        ComponentMapping, ComponentSwizzle, CompositeAlphaFlagsKHR, Extent3D, Format, Image, ImageAspectFlags, ImageUsageFlags, ImageView, ImageViewCreateInfo, ImageViewType, MemoryPropertyFlags, SharingMode, SwapchainCreateInfoKHR, SwapchainKHR
    },
};
use vk_mem::{Alloc, MemoryUsage};
use winit::window::Window;

use super::{
    allocated_image::AllocatedImage, device::VkDevice, image_util::{image_create_info, image_subresource_range, image_view_create_info}, instance::VkInstance, queue::VkQueue, swapchain_support_details::SwapchainSupportDetails
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ImageDetails {
    pub image: Image,
    pub image_view: ImageView,
}

#[derive(Clone)]
#[allow(unused)]
pub struct KHRSwapchain {
    pub s_device: swapchain::Device,
    swapchain: SwapchainKHR,
    pub details: SwapchainSupportDetails,
    pub device: Arc<VkDevice>,
    instance: Arc<VkInstance>,
}

impl Deref for KHRSwapchain {
    type Target = SwapchainKHR;

    fn deref(&self) -> &Self::Target {
        &self.swapchain
    }
}

impl KHRSwapchain {
    pub fn new(
        instance: Arc<VkInstance>,
        device: Arc<VkDevice>,
        surface: Arc<super::surface::KHRSurface>,
        window: &Window,
        queues: [Arc<VkQueue>; 2],
    ) -> Result<Self, Error> {
        let s_device = swapchain::Device::new(&instance, &device);
        let swapchain_support_details = SwapchainSupportDetails::get_swapchain_support_details(
            device.physical_device,
            surface.clone(),
            window,
        )
        .unwrap();
        let surface_format = swapchain_support_details.clone().choose_swapchain_format();
        let present_mode = swapchain_support_details
            .clone()
            .choose_swapchain_present_mode();
        let extent = swapchain_support_details
            .clone()
            .choose_swapchain_extent(window);
        let image_count = swapchain_support_details.clone().choose_image_count();

        let mut create_info = SwapchainCreateInfoKHR::default()
            .surface(**surface)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(ImageUsageFlags::COLOR_ATTACHMENT | ImageUsageFlags::TRANSFER_DST)
            .pre_transform(swapchain_support_details.capabilities.current_transform)
            .composite_alpha(CompositeAlphaFlagsKHR::OPAQUE)
            .clipped(true)
            .present_mode(present_mode)
            .image_extent(extent);

        let indices_vec = [queues[0].queue_family_index, queues[1].queue_family_index];
        if indices_vec[0] != indices_vec[1] {
            create_info = create_info
                .image_sharing_mode(SharingMode::CONCURRENT)
                .queue_family_indices(&indices_vec);
        } else {
            create_info = create_info.image_sharing_mode(SharingMode::EXCLUSIVE);
        }

        let swapchain = unsafe { s_device.create_swapchain(&create_info, None).unwrap() };

        Ok(Self {
            swapchain,
            s_device,
            device,
            instance,
            details: swapchain_support_details,
        })
    }
    

    pub fn create_image_details(&self) -> Result<Vec<ImageDetails>, Error> {
        unsafe {
            let images = self.s_device.get_swapchain_images(self.swapchain).unwrap();
            let image_details = images
                .into_iter()
                .map(|image| -> ImageDetails {
                    let image_view_create_info = ImageViewCreateInfo::default()
                        .image(image)
                        .format(
                            self.details
                                .clone()
                                .choose_swapchain_format()
                                .format,
                        )
                        .subresource_range(image_subresource_range(ImageAspectFlags::COLOR))
                        .view_type(ImageViewType::TYPE_2D)
                        .components(
                            ComponentMapping::default()
                                .r(ComponentSwizzle::IDENTITY)
                                .g(ComponentSwizzle::IDENTITY)
                                .b(ComponentSwizzle::IDENTITY)
                                .a(ComponentSwizzle::IDENTITY),
                        );

                    let image_view = self
                        .device
                        .create_image_view(&image_view_create_info, None)
                        .unwrap();
                    ImageDetails { image, image_view }
                })
                .collect::<Vec<ImageDetails>>();
            Ok(image_details)
        }
    }
}
