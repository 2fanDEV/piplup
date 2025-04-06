use std::{env, ffi::CStr, io::Error, ops::Deref, sync::Arc};

use ash::{
    vk::{ApplicationInfo, InstanceCreateInfo, API_VERSION_1_3},
    Entry, Instance, LoadingError,
};
use winit::{raw_window_handle::HasDisplayHandle, window::Window};

pub fn load_vulkan_library() -> Result<Entry, LoadingError> {
    #[cfg(target_os = "macos")]
    let entry_path = env::home_dir().unwrap().to_str().unwrap().to_owned()
        + "/VulkanSDK/1.4.309.0/macOS/lib/libvulkan.dylib";
    Ok(unsafe { Entry::load_from(entry_path)? })
}

pub struct VkInstance {
    entry: Entry,
    instance: Arc<Instance>,
}

impl Deref for VkInstance {
    type Target = Instance;

    fn deref(&self) -> &Self::Target {
        &self.instance
    }
}

impl VkInstance {
    pub fn new(window: Window) -> Result<VkInstance, Error> {
        let entry = load_vulkan_library().unwrap();
        let application_info = Self::app_create_info(c"PULPIP", c"PIPLUP");
        let mut required_extensions = ash_window::enumerate_required_extensions(window.display_handle().unwrap().as_raw()).unwrap().to_vec();
        required_extensions.push(ash::khr::portability_subset::NAME.as_ptr());

        let instance = unsafe {
            entry.clone()
                .create_instance(&Self::instance_create_info(entry.clone(),&application_info, &required_extensions), None)
                .unwrap()
        };
        Ok(Self {
            entry,
            instance: Arc::new(instance),
        })
    }

    fn instance_create_info<'a>(entry: Entry, app_info: &'a ApplicationInfo, required_extensions: &'a [*const i8]) -> InstanceCreateInfo<'a> {
        let create_info =  InstanceCreateInfo::default().application_info(app_info)
            .enabled_extension_names(required_extensions);
        create_info
    }

    fn app_create_info<'a>(engine_name: &'a CStr, app_name: &'a CStr) -> ApplicationInfo<'a> {
        ApplicationInfo::default()
            .engine_name(engine_name)
            .api_version(API_VERSION_1_3)
            .application_name(app_name)
    }
}
