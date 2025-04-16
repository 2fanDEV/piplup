use std::{io::Error, ops::Deref, sync::Arc};

use ash::vk::{
    DescriptorImageInfo, DescriptorPool, DescriptorPoolCreateFlags, DescriptorPoolCreateInfo,
    DescriptorPoolResetFlags, DescriptorPoolSize, DescriptorSet, DescriptorSetAllocateInfo,
    DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateFlags,
    DescriptorSetLayoutCreateInfo, DescriptorType, Image, ImageLayout, ImageView, Sampler,
    SamplerCreateInfo, ShaderStageFlags, WriteDescriptorSet,
};

use super::{allocated_image::AllocatedImage, device::VkDevice, sampler::VkSampler};

pub struct DescriptorSetDetails {
    descriptor_set: DescriptorSet,
    pub layout: DescriptorSetLayout,
}

impl Deref for DescriptorSetDetails {
    type Target = DescriptorSet;

    fn deref(&self) -> &Self::Target {
        &self.descriptor_set
    }
}

pub struct DescriptorAllocator {
    pool: DescriptorPool,
    device: Arc<VkDevice>,
}

pub struct PoolSizeRatio {
    descriptor_type: DescriptorType,
    ratio: f32,
}

pub struct DescriptorLayoutBuilder<'a> {
    bindings: Vec<DescriptorSetLayoutBinding<'a>>,
}

impl PoolSizeRatio {
    pub fn new(descriptor_type: DescriptorType, ratio: f32) -> PoolSizeRatio {
        Self {
            descriptor_type,
            ratio,
        }
    }
}

impl DescriptorAllocator {
    pub fn new(
        device: Arc<VkDevice>,
        max_sets: u32,
        pool_sizes: Vec<PoolSizeRatio>,
    ) -> DescriptorAllocator {
        let mut descriptor_pool_sizes: Vec<DescriptorPoolSize> = vec![];
        for pool_size in pool_sizes {
            descriptor_pool_sizes.push(
                DescriptorPoolSize::default()
                    .ty(pool_size.descriptor_type)
                    .descriptor_count(pool_size.ratio as u32 * max_sets),
            );
        }
        let create_info = DescriptorPoolCreateInfo::default()
            .max_sets(max_sets)
            .pool_sizes(&descriptor_pool_sizes)
            .flags(DescriptorPoolCreateFlags::empty());

        Self {
            device: device.clone(),
            pool: unsafe { device.create_descriptor_pool(&create_info, None).unwrap() },
        }
    }

    pub fn get_descriptors(
        &self,
        image_view: ImageView,
        shader_stage: ShaderStageFlags,
        descriptor_type: DescriptorType,
        sampler: Option<VkSampler>,
    ) -> Result<DescriptorSetDetails, Error> {
        let mut descriptor_layout_builder = DescriptorLayoutBuilder::new();
        descriptor_layout_builder.add_binding(0, descriptor_type);
        let layout = descriptor_layout_builder.build(
            self.device.clone(),
            shader_stage,
            DescriptorSetLayoutCreateFlags::empty(),
        );

        let descriptor_set = self.allocate(self.device.clone(), &[layout]);
        let mut descriptor_image_infos = vec![];
        let mut descriptor_info = DescriptorImageInfo::default()
            .image_layout(ImageLayout::GENERAL)
            .image_view(image_view);
        if sampler.is_some() {
            descriptor_info = descriptor_info.sampler(*sampler.unwrap());
        }
        descriptor_image_infos.push(descriptor_info);

        let write_descriptor_set = WriteDescriptorSet::default()
            .dst_binding(0)
            .descriptor_count(1)
            .dst_set(descriptor_set)
            .descriptor_type(descriptor_type)
            .image_info(&descriptor_image_infos);

        unsafe {
            self.device
                .update_descriptor_sets(&[write_descriptor_set], &[])
        };

        Ok(DescriptorSetDetails {
            descriptor_set,
            layout,
        })
    }

    pub fn reset_descriptors(&self, device: Arc<VkDevice>) {
        unsafe {
            device
                .reset_descriptor_pool(self.pool, DescriptorPoolResetFlags::empty())
                .unwrap()
        }
    }

    pub fn destroy_pool(&self, device: Arc<VkDevice>) {
        unsafe { device.destroy_descriptor_pool(self.pool, None) }
    }

    fn allocate(&self, device: Arc<VkDevice>, layouts: &[DescriptorSetLayout]) -> DescriptorSet {
        let mut allocate_info = DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.pool)
            .set_layouts(layouts);
        allocate_info.descriptor_set_count = 1;

        unsafe {
            *device
                .allocate_descriptor_sets(&allocate_info)
                .unwrap()
                .get(0)
                .unwrap()
        }
    }
}

impl<'a> DescriptorLayoutBuilder<'a> {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    pub fn add_binding(&mut self, binding: u32, descriptor_type: DescriptorType) {
        let descriptor_binding = DescriptorSetLayoutBinding::default()
            .binding(binding)
            .descriptor_type(descriptor_type)
            .descriptor_count(1);

        self.bindings.push(descriptor_binding);
    }

    pub fn clear(&mut self) {
        self.bindings.clear();
    }

    pub fn build(
        &mut self,
        device: Arc<VkDevice>,
        shader_stages: ShaderStageFlags,
        flags: DescriptorSetLayoutCreateFlags,
    ) -> DescriptorSetLayout {
        for binding in &mut self.bindings {
            binding.stage_flags = binding.stage_flags | shader_stages
        }

        let descriptor_set_create_info = DescriptorSetLayoutCreateInfo::default()
            .bindings(&self.bindings)
            .flags(flags);

        unsafe {
            device
                .create_descriptor_set_layout(&descriptor_set_create_info, None)
                .unwrap()
        }
    }
}
