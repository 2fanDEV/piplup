use std::{
    env,
    ffi::{c_void, CStr},
    io::Error,
    ops::{Add, Deref},
    sync::Arc,
};

use ash::{
    ext::debug_utils,
    vk::{
        ApplicationInfo, DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessageTypeFlagsEXT, DebugUtilsMessengerCallbackDataEXT, DebugUtilsMessengerCreateInfoEXT, DebugUtilsMessengerEXT, InstanceCreateFlags, InstanceCreateInfo, API_VERSION_1_3, EXT_DEBUG_UTILS_NAME
    },
    Entry, Instance, LoadingError,
};
use log::{debug, info, warn};
use winit::{raw_window_handle::HasDisplayHandle, window::Window};

pub fn load_vulkan_library() -> Result<Entry, LoadingError> {
    #[cfg(target_os = "macos")]
    let entry_path = env::home_dir().unwrap().to_str().unwrap().to_owned()
        + "/VulkanSDK/1.4.309.0/macOS/lib/libvulkan.dylib";
    Ok(unsafe { Entry::load_from(entry_path)? })
}

#[derive(Clone)]
pub struct VkInstance {
    pub entry: Entry,
    pub instance: Instance,
}


impl Deref for VkInstance {
    type Target = Instance;

    fn deref(&self) -> &Self::Target {
        &self.instance
    }
}

impl VkInstance {
    pub fn new(window: &Window) -> Result<VkInstance, Error> {
        let entry = load_vulkan_library().unwrap();
        let application_info = Self::app_create_info(c"PULPIP", c"PIPLUP");
        let mut required_extensions =
            ash_window::enumerate_required_extensions(window.display_handle().unwrap().as_raw())
                .unwrap()
                .to_vec();
        required_extensions.push(ash::khr::portability_enumeration::NAME.as_ptr());

        let extension_properties = unsafe {
            entry
                .enumerate_instance_extension_properties(None)
                .unwrap()
                .iter()
                .map(|f| {
                    f.extension_name_as_c_str()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string()
                })
                .collect::<Vec<String>>()
        };

        debug!(
            "Loaded {} instance extension properties: {extension_properties:#?}",
            extension_properties.len()
        );

        let mut debug_create_info = DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | DebugUtilsMessageSeverityFlagsEXT::INFO
                    | DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    | DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(
                DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(Self::debug_callback));

        let validation_layers = unsafe {
            entry
                .enumerate_instance_layer_properties()
                .unwrap()
                .iter()
                .map(|layer| {
                    layer
                        .layer_name_as_c_str()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string()
                })
                .collect::<Vec<String>>()
        };

        let enabled_layer_support = Self::check_validation_layers(validation_layers);
        if enabled_layer_support {
            debug!("ENABLED LAYER SUPPORT");
            required_extensions.push(EXT_DEBUG_UTILS_NAME.as_ptr());
        }

        let instance = unsafe {
            entry
                .create_instance(
                    &Self::instance_create_info(
                        &application_info,
                        &required_extensions,
                        enabled_layer_support,
                        &mut debug_create_info,
                    ),
                    None,
                )
                .unwrap()
        };
        Ok(Self {
            entry,
            instance: instance,
        })
    }

    fn instance_create_info<'a>(
        app_info: &'a ApplicationInfo,
        required_extensions: &'a [*const i8],
        layers_enabled: bool,
        debug_create_info: &'a mut DebugUtilsMessengerCreateInfoEXT<'a>,
    ) -> InstanceCreateInfo<'a> {
        let mut create_info = InstanceCreateInfo::default()
            .application_info(app_info)
            .flags(InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR)
            .enabled_extension_names(required_extensions);
        
        if layers_enabled {
            create_info = create_info.push_next(debug_create_info);
        }

        create_info
    }

    fn app_create_info<'a>(engine_name: &'a CStr, app_name: &'a CStr) -> ApplicationInfo<'a> {
        ApplicationInfo::default()
            .engine_name(engine_name)
            .api_version(API_VERSION_1_3)
            .application_name(app_name)
    }

    pub fn create_debugger(
        instance: Arc<VkInstance>
    ) -> (debug_utils::Instance, DebugUtilsMessengerEXT) {
        let  debug_create_info = DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | DebugUtilsMessageSeverityFlagsEXT::INFO
                    | DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    | DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(
                DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | DebugUtilsMessageTypeFlagsEXT::VALIDATION | DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(Self::debug_callback)); 
        let debug_instance = debug_utils::Instance::new(&instance.entry, &instance);
        let debugger = unsafe {
            debug_instance
                .create_debug_utils_messenger(&debug_create_info, None)
                .unwrap()
        };
        (debug_instance, debugger)
    }

    fn check_validation_layers(validation_layers: Vec<String>) -> bool {
        let validation_layer_tbc = vec![String::from("VK_LAYER_KHRONOS_validation")];
        let mut count = 0;

        for layer in validation_layer_tbc.clone() {
            if validation_layers.contains(&layer) {
                count = count.add(1);
            }
        }
        debug!("{count:?}, {:?}", validation_layer_tbc.len());
        count == validation_layer_tbc.len()
    }

    unsafe extern "system" fn debug_callback(
        message_severity: DebugUtilsMessageSeverityFlagsEXT,
        message_type: DebugUtilsMessageTypeFlagsEXT,
        callback_data: *const DebugUtilsMessengerCallbackDataEXT<'_>,
        user_data: *mut c_void,
    ) -> u32 {
        unsafe {
            let p_callback_data = *callback_data;
            let message_id_name = p_callback_data
                .message_id_name_as_c_str()
                .unwrap()
                .to_string_lossy();
            let message_id_number = p_callback_data.message_id_number;
            let message = p_callback_data
                .message_as_c_str()
                .unwrap()
                .to_string_lossy();

            match message_severity {
                DebugUtilsMessageSeverityFlagsEXT::WARNING => {
                    warn!(
                        "{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n"
                    );
                }
                DebugUtilsMessageSeverityFlagsEXT::ERROR => {
                    log::error!(
                        "{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n"
                    )
                }
                _ => {
                    info!(
                        "{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n"
                    );
                }
                _ => {
                    info!(
                        "{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n"
                    );
                }
            }
        }
        0
    }
}
