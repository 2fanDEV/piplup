use std::{ops::Deref, sync::Arc};

use ash::vk::{
    BorderColor, CompareOp, Filter, Sampler, SamplerAddressMode, SamplerCreateInfo,
    SamplerMipmapMode,
};

use super::device::VkDevice;

#[derive(Clone)]
pub struct VkSampler {
    sampler: Sampler,
    _device: Arc<VkDevice>,
}

impl Deref for VkSampler {
    type Target = Sampler;

    fn deref(&self) -> &Self::Target {
        &self.sampler
    }
}

impl VkSampler {
    pub fn get_font_sampler(device: Arc<VkDevice>) -> VkSampler {
        let properties = unsafe {
            device
                .instance
                .get_physical_device_properties(device.physical_device)
        };
        let create_info = SamplerCreateInfo::default()
            .mag_filter(Filter::LINEAR)
            .min_filter(Filter::LINEAR)
            // REPEAT NOT FOR UI FONTS
            .address_mode_u(SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(SamplerAddressMode::CLAMP_TO_EDGE)
            .anisotropy_enable(true)
            .max_anisotropy(properties.limits.max_sampler_anisotropy)
            .border_color(BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_op(CompareOp::ALWAYS)
            .compare_enable(false)
            .mipmap_mode(SamplerMipmapMode::NEAREST)
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(0.0);
        Self {
            sampler: unsafe { device.create_sampler(&create_info, None).unwrap() },
            _device: device,
        }
    }
}
