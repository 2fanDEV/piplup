use std::{io::Error, sync::Arc};

use ash::
    vk::{
        ColorSpaceKHR, Extent2D, Format, PhysicalDevice, PresentModeKHR, SurfaceCapabilitiesKHR,
        SurfaceFormatKHR,
    }
;
use log::debug;
use muda::dpi::PhysicalSize;
use winit::window::Window;

use super::surface::KHRSurface;

#[derive(Default, Clone)]
pub struct SwapchainSupportDetails {
    pub capabilities: SurfaceCapabilitiesKHR,
    formats: Vec<SurfaceFormatKHR>,
    present_modes: Vec<PresentModeKHR>,
    pub window_sizes: PhysicalSize<u32>
}

impl SwapchainSupportDetails {
    pub fn get_swapchain_support_details(
        physical_device: PhysicalDevice,
        surface: Arc<KHRSurface>,
        window: &Window
    ) -> Result<SwapchainSupportDetails, Error> {
        let surface_capabilities = unsafe {
            surface.instance
                .get_physical_device_surface_capabilities(physical_device, **surface)
                .unwrap()
        };
        let formats = unsafe {
            surface.instance
                .get_physical_device_surface_formats(physical_device, **surface)
                .unwrap()
        };
        let present_modes = unsafe {
            surface.instance
                .get_physical_device_surface_present_modes(physical_device, **surface)
                .unwrap()
        };

        let window_sizes = window.inner_size();
        Ok(Self {
            capabilities: surface_capabilities,
            formats,
            present_modes,
            window_sizes
        })
    }

    pub fn is_swapchain_adequate(self) -> bool {
        !self.formats.is_empty() && !self.present_modes.is_empty()
    }

    pub fn choose_swapchain_format(self) -> SurfaceFormatKHR {
        self.formats
            .into_iter()
            .filter(|format| {
                format.format.eq(&Format::R16G16B16A16_SFLOAT) 
                    && format.color_space.eq(&ColorSpaceKHR::SRGB_NONLINEAR)
            })
            .collect::<Vec<SurfaceFormatKHR>>()
            .first().copied()
            .unwrap()
    }

    pub fn choose_swapchain_present_mode(self) -> PresentModeKHR {
        self.present_modes
            .into_iter()
            .filter(|mode| mode.eq(&PresentModeKHR::FIFO))
            .collect::<Vec<PresentModeKHR>>()
            .first().copied()
            .unwrap()
    }

    pub fn choose_swapchain_extent(self, window: &Window) -> Extent2D {
        let mut current_extent = self.capabilities.current_extent;
        if current_extent.width != u32::MAX {
            current_extent
        } else {
            let size = window.inner_size();
            current_extent = current_extent
                .width(size.width.clamp(
                    self.capabilities.min_image_extent.width,
                    self.capabilities.max_image_extent.width,
                ))
                .height(size.height.clamp(
                    self.capabilities.min_image_extent.height,
                    self.capabilities.max_image_extent.height,
                ));
            current_extent
        }
    }

    pub fn choose_image_count(self) -> u32 {
        let min_image_count = self.capabilities.min_image_count + 1;
        let max_image_count = self.capabilities.max_image_count;
        if max_image_count > 0 && min_image_count > max_image_count {
            return max_image_count;
        }
        min_image_count
    }
}
