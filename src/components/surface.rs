use std::{io::Error, ops::Deref, sync::Arc};

use ash::{khr::surface, vk::SurfaceKHR};
use winit::{raw_window_handle::{HasDisplayHandle, HasWindowHandle}, window::Window};

use super::instance::VkInstance;

pub struct KHRSurface {
    pub instance: surface::Instance,
    pub surface_khr: SurfaceKHR,
}

impl Deref for KHRSurface {
    type Target = SurfaceKHR;

    fn deref(&self) -> &Self::Target {
        &self.surface_khr
    }
}

impl KHRSurface {
    pub fn new(instance: Arc<VkInstance>, window: &Window) -> Result<Self, Error> {
        let surface_instance = surface::Instance::new(&instance.entry, &instance);
        let surface_khr = unsafe {
            ash_window::create_surface(
                &instance.entry,
                &instance,
                window.display_handle().unwrap().as_raw(),
                window.window_handle().as_ref().unwrap().as_raw(),
                None,
            )
            .unwrap()
        };

        Ok(Self {
            instance: surface_instance,
            surface_khr
        })
    }
}
