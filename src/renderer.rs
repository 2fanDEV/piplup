use std::{io::Error, sync::Arc};

use ash::{ext::debug_utils, vk::DebugUtilsMessengerEXT};
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

use crate::components::{
    device::{self, VkDevice},
    instance::{self, VkInstance}, queue::{QueueType, VkQueue}, surface,
};

pub struct Renderer {
    instance: Arc<VkInstance>,
    debug_instance: debug_utils::Instance,
    debugger: DebugUtilsMessengerEXT,
    device: Arc<VkDevice>,
}

impl Renderer {
    pub fn init(window: &Window) -> Result<Renderer, Error> {
        let vk_instance = Arc::new(instance::VkInstance::new(window)?);
        let (debug_instance, debugger) = instance::VkInstance::create_debugger(vk_instance.clone());
        let surface = Arc::new(surface::KHRSurface::new(vk_instance.clone(), window)?);
                let vk_device =
            Arc::new(device::VkDevice::new(vk_instance.clone(), surface.clone(), window)?);
        let graphics_queue =  VkQueue::new(vk_device.clone(), surface.clone(), QueueType::GRAPHICS_QUEUE);
        Ok(Self {
            instance: vk_instance,
            debug_instance,
            debugger,
            device: vk_device,
        })
    }

    pub fn draw(&self) {}
}
