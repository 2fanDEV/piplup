use std::{io::Error, ops::Deref, sync::Arc};

use anyhow::{anyhow, Result};
use ash::{
    vk::{
        DescriptorImageInfo, DescriptorPool, DescriptorPoolCreateFlags, DescriptorPoolCreateInfo,
        DescriptorPoolResetFlags, DescriptorPoolSize, DescriptorSet, DescriptorSetAllocateInfo,
        DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateFlags,
        DescriptorSetLayoutCreateInfo, DescriptorType, ImageLayout, ImageView, ShaderStageFlags,
        WriteDescriptorSet,
    },
    Device,
};

use super::{device::VkDevice, sampler::VkSampler};

#[derive(Debug)]
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
    device: Arc<VkDevice>,
    ratios: Vec<PoolSizeRatio>,
    full_pools: Vec<DescriptorPool>,
    ready_pools: Vec<DescriptorPool>,
    sets_per_pool: u32,
}

#[derive(Copy, Clone)]
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
        let mut ready_pools = vec![];
        let full_pools = vec![];
        let pool = Self::create_pool(&device, max_sets, &pool_sizes).unwrap();
        ready_pools.push(pool);

        Self {
            device: device.clone(),
            ratios: pool_sizes,
            full_pools,
            ready_pools,
            sets_per_pool: (max_sets as f32 * 1.5) as u32,
        }
    }

    fn get_pool(&mut self) -> Result<DescriptorPool> {
        let mut new_pool: Option<DescriptorPool> = None;
        if !self.ready_pools.is_empty() {
            new_pool = Some(self.ready_pools.pop().unwrap());
        } else {
            new_pool = Some(Self::create_pool(
                &self.device,
                self.sets_per_pool,
                &self.ratios,
            )?)
        }
        Ok(new_pool.unwrap())
    }

    fn create_pool(
        device: &Device,
        set_count: u32,
        pool_sizes: &[PoolSizeRatio],
    ) -> Result<DescriptorPool> {
        let mut descriptor_pool_sizes = vec![];
        for pool_size in pool_sizes {
            descriptor_pool_sizes.push(
                DescriptorPoolSize::default()
                    .ty(pool_size.descriptor_type)
                    .descriptor_count(pool_size.ratio as u32 * set_count),
            );
        }
        let create_info = DescriptorPoolCreateInfo::default()
            .max_sets(set_count)
            .pool_sizes(&descriptor_pool_sizes)
            .flags(DescriptorPoolCreateFlags::empty());

        unsafe { Ok(device.create_descriptor_pool(&create_info, None).unwrap()) }
    }

    pub fn write_descriptors(
        &mut self,
        image_view: &ImageView,
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
            .image_layout(ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(*image_view);
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

    pub fn reset_descriptors(&mut self, device: Arc<VkDevice>) {
        unsafe {
            for pool in &self.ready_pools {
                device
                    .reset_descriptor_pool(*pool, DescriptorPoolResetFlags::empty())
                    .unwrap()
            }

            for pool in &self.full_pools {
                device
                    .reset_descriptor_pool(*pool, DescriptorPoolResetFlags::empty())
                    .unwrap();
                self.ready_pools.push(*pool);
            }

            self.full_pools.clear();
        }
    }

    pub fn destroy_pools(&mut self, device: Arc<VkDevice>) {
        for pool in &self.ready_pools {
            unsafe { device.destroy_descriptor_pool(*pool, None) }
        }
        self.ready_pools.clear();
        for pool in &self.full_pools {
            unsafe { device.destroy_descriptor_pool(*pool, None) }
        }
        self.full_pools.clear();
    }

    fn allocate(
        &mut self,
        device: Arc<VkDevice>,
        layouts: &[DescriptorSetLayout],
    ) -> DescriptorSet {
        let mut pool_to_use = self.get_pool().unwrap();
        let mut allocate_info = DescriptorSetAllocateInfo::default()
            .descriptor_pool(pool_to_use)
            .set_layouts(layouts);
        allocate_info.descriptor_set_count = 1;

        let descriptor_sets = match unsafe { device.allocate_descriptor_sets(&allocate_info) } {
            Ok(sets) => sets,
            Err(error) => {
                if error == ash::vk::Result::ERROR_OUT_OF_POOL_MEMORY
                    || error == ash::vk::Result::ERROR_FRAGMENTED_POOL
                {
                    if !self.full_pools.contains(&pool_to_use) {
                        self.full_pools.push(pool_to_use);
                    };
                    pool_to_use = self.get_pool().unwrap();
                    allocate_info.descriptor_pool = pool_to_use;
                    unsafe { device.allocate_descriptor_sets(&allocate_info).unwrap() }
                } else {
                    panic!("Something else went wrong here");
                }
            }
        };
        descriptor_sets[0]
    }
}

impl Default for DescriptorLayoutBuilder<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl DescriptorLayoutBuilder<'_> {
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
            binding.stage_flags |= shader_stages
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
