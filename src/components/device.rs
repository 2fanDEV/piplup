use std::{io::Error, ops::Deref, sync::Arc};

use ash::{
    khr::surface,
    vk::{DeviceCreateInfo, DeviceQueueCreateInfo, PhysicalDevice, QueueFlags, SurfaceKHR, KHR_PORTABILITY_SUBSET_NAME, KHR_SWAPCHAIN_NAME},
    Device, Instance,
};
use log::error;
use winit::window::Window;

use super::{instance::VkInstance, surface::KHRSurface, swapchain_support_details::SwapchainSupportDetails};

#[derive(Default, Clone, Copy)]
pub struct QueueFamilyIndices {
    pub graphics_q_idx: Option<u32>,
    pub presentation_q_idx: Option<u32>,
}

impl QueueFamilyIndices {
    pub fn find_queue_family_indices(
        physical_device: PhysicalDevice,
        instance: &Instance,
        surface: Arc<KHRSurface>
    ) -> QueueFamilyIndices {
        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let mut indices = QueueFamilyIndices {
            graphics_q_idx: None,
            presentation_q_idx: None,
        };

        for (idx, property) in queue_family_properties.iter().enumerate() {
            if property.queue_flags.contains(QueueFlags::GRAPHICS) {
                indices.graphics_q_idx = Some(idx as u32);
                let surface_support = unsafe {
                    surface.instance
                        .get_physical_device_surface_support(physical_device, idx as u32, **surface)
                        .unwrap()
                };
                if surface_support {
                    indices.presentation_q_idx = Some(idx as u32);
                }
            }
        }
        indices
    }

    pub fn is_complete(&self) -> bool {
        self.graphics_q_idx.is_some() && self.presentation_q_idx.is_some()
    }
}

pub struct VkDevice {
    pub device: Device,
    pub physical_device: PhysicalDevice,
    pub instance: Instance
}

impl Deref for VkDevice {
    type Target = Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl VkDevice {
    pub fn new(
        instance: Arc<VkInstance>,
        surface: Arc<KHRSurface>,
        window: &Window,
    ) -> Result<VkDevice, Error> {
        let physical_device = Self::pick_physical_device(&instance, &surface, window);
        let device = match Self::create_device(
            &instance,
            physical_device,
            surface,
            window,
        ) {
            Some(device) => device,
            None => panic!()
        };
        Ok(Self {
            physical_device: physical_device.unwrap(),
            device,
            instance: instance.instance.clone()
        })
    }

    pub fn create_device(
        instance: &VkInstance,
        physical_device: Option<PhysicalDevice>,
        surface: Arc<KHRSurface>,
        window: &Window,
    ) -> Option<ash::Device> {
        match physical_device {
            Some(physical_device) => {
                let indices = QueueFamilyIndices::find_queue_family_indices(
                    physical_device,
                    instance,
                    surface
                );
                let features = unsafe { instance.get_physical_device_features(physical_device) };
                let extensions = vec![
                    KHR_SWAPCHAIN_NAME.as_ptr(),
                    KHR_PORTABILITY_SUBSET_NAME.as_ptr(),
                ];

                let device_queue_create_infos = vec![DeviceQueueCreateInfo::default()
                    .queue_family_index(indices.graphics_q_idx.unwrap())
                    .queue_priorities(&[1.0])];
                let device_create_infos = DeviceCreateInfo::default()
                    .enabled_features(&features)
                    .queue_create_infos(&device_queue_create_infos)
                    .enabled_features(&features)
                    .enabled_extension_names(&extensions);
                let device = unsafe {
                    instance
                        .create_device(physical_device, &device_create_infos, None)
                        .ok()
                };
                Some(device?)
            }
            None => None,
        }
    }

    fn pick_physical_device(
        instance: &VkInstance,
        surface: &Arc<KHRSurface>,
        window: &Window,
    ) -> Option<PhysicalDevice> {
        match unsafe { instance.enumerate_physical_devices() } {
            Ok(devices) => {
                devices
                    .into_iter()
                    .filter(|device| {
                        Self::is_device_suitable(*device, &instance, surface.clone(), window)
                    })
                    .collect::<Vec<PhysicalDevice>>()
                    .first()
                    .map(|dev| dev.to_owned()) // we want an owned value to return
            }
            Err(_) => {
                error!("Failed to pick a physical device!");
                None
            }
        }
    }

    fn check_device_extensions(device: PhysicalDevice, instance: &VkInstance) -> bool {
        let extensions = vec![KHR_SWAPCHAIN_NAME.to_str().unwrap().to_string()];
        let p_device_extensions = unsafe {
            instance
                .enumerate_device_extension_properties(device)
                .unwrap()
                .iter()
                .map(|extension| {
                    extension
                        .extension_name_as_c_str()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string()
                })
                .collect::<Vec<String>>()
        };
        let mut count = 0;
        for extension in &extensions {
            if p_device_extensions.contains(&extension) {
                count = count + 1;
            }
        }
        extensions.len() == count
    }

    fn is_device_suitable(
        device: PhysicalDevice,
        instance: &VkInstance,
        surface: Arc<KHRSurface>,
        window: &Window,
    ) -> bool {
        let queue_family_indices = QueueFamilyIndices::find_queue_family_indices(
            device,
            instance,
            surface.clone()
        );
        let features = unsafe { instance.get_physical_device_features(device) };
        let properties = unsafe { instance.get_physical_device_properties(device) };
        let swapchain_support_details = SwapchainSupportDetails::get_swapchain_support_details(
            device,
            surface.clone(),
            window,
        )
        .unwrap();
        queue_family_indices.is_complete()
            && Self::check_device_extensions(device, instance)
            && swapchain_support_details.is_swapchain_adequate()
    }
}
