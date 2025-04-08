use std::{io::Error, ops::Deref, sync::Arc};

use ash::{khr::swapchain, vk::SwapchainKHR};

use super::{device::VkDevice, instance::VkInstance};


pub struct KHRSwapchain {
    s_device: swapchain::Device,
    swapchain: SwapchainKHR,
    device: Arc<VkDevice>,
    instance: Arc<VkInstance>
}

impl Deref for KHRSwapchain {
    type Target = SwapchainKHR;

    fn deref(&self) -> &Self::Target {
        &self.swapchain
    }
}

impl KHRSwapchain {
    
    pub fn new(instance: Arc<VkInstance>, device: Arc<VkDevice>) -> Result<Self, Error> {
       let s_device = swapchain::Device::new(&instance, &device);
       s_device.create_swapchain(create_info, allocation_callbacks)
    }
}
