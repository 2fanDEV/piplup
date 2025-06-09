use std::{
    io::Error,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use anyhow::Result;
use ash::{
    vk::{
        DescriptorBufferInfo, DescriptorImageInfo, DescriptorPool, DescriptorPoolCreateFlags,
        DescriptorPoolCreateInfo, DescriptorPoolResetFlags, DescriptorPoolSize, DescriptorSet,
        DescriptorSetAllocateInfo, DescriptorSetLayout, DescriptorSetLayoutBinding,
        DescriptorSetLayoutCreateFlags, DescriptorSetLayoutCreateInfo, DescriptorType, ImageLayout,
        ImageView, ShaderStageFlags, WriteDescriptorSet,
    },
    Device,
};
use log::debug;

use super::{allocation_types::VkBuffer, device::VkDevice, sampler::VkSampler};


#[derive(Debug, Clone, Default)]
pub struct DescriptorSetDetails {
    descriptor_set: Vec<DescriptorSet>,
    pub layout: Vec<DescriptorSetLayout>,
}

impl Deref for DescriptorSetDetails {
    type Target = Vec<DescriptorSet>;

    fn deref(&self) -> &Self::Target {
        &self.descriptor_set
    }
}

#[derive(Debug, Clone, Copy)]
struct PendingImageWrite {
    binding: u32,
    image_info_idx: usize,
    d_type: DescriptorType,
}

#[derive(Debug, Clone, Copy)]
struct PendingBufferWrite {
    binding: u32,
    buffer_info_idx: usize,
    d_type: DescriptorType,
}

#[derive(Clone, Debug)]
pub struct DescriptorWriter {
    image_infos: Vec<DescriptorImageInfo>,
    buffer_infos: Vec<DescriptorBufferInfo>,
    pending_image_writes: Vec<PendingImageWrite>,
    pending_buffer_writes: Vec<PendingBufferWrite>,
}

impl Default for DescriptorWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl DescriptorWriter {
    pub fn new() -> Self {
        Self {
            image_infos: vec![],
            buffer_infos: vec![],
            pending_image_writes: vec![],
            pending_buffer_writes: vec![],
        }
    }

    pub fn write_image(
        &mut self,
        binding: u32,
        image: ImageView,
        sampler: Option<VkSampler>,
        layout: ImageLayout,
        d_type: DescriptorType,
    ) {
        let mut descriptor_image_info = DescriptorImageInfo::default()
            .image_view(image)
            .image_layout(layout);
        descriptor_image_info = match sampler {
            Some(sampler) => descriptor_image_info.sampler(*sampler),
            None => descriptor_image_info,
        };
        self.image_infos.push(descriptor_image_info);

        self.pending_image_writes.push(PendingImageWrite {
            binding,
            image_info_idx: self.image_infos.len() - 1,
            d_type,
        })
    }

    pub fn write_buffer(
        &mut self,
        binding: u32,
        buffer: VkBuffer,
        size: u64,
        offset: u64,
        d_type: DescriptorType,
    ) {
        let descriptor_buffer_info = DescriptorBufferInfo::default()
            .buffer(*buffer)
            .offset(offset)
            .range(size);
        self.buffer_infos.push(descriptor_buffer_info);

        self.pending_buffer_writes.push(PendingBufferWrite {
            binding,
            buffer_info_idx: self.buffer_infos.len() - 1,
            d_type,
        })
    }

    pub fn update_set(&mut self, device: Arc<VkDevice>, set: DescriptorSet) {
        let mut writes: Vec<WriteDescriptorSet> = vec![];
        for image_write in &self.pending_image_writes {
            let image_ref = &self.image_infos[image_write.image_info_idx];
            writes.push(
                WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(image_write.binding)
                    .descriptor_type(image_write.d_type)
                    .image_info(std::slice::from_ref(image_ref)),
            )
        }

        for buffer_write in &self.pending_buffer_writes {
            let buffer_ref = &self.buffer_infos[buffer_write.buffer_info_idx];
            writes.push(
                WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(buffer_write.binding)
                    .descriptor_type(buffer_write.d_type)
                    .buffer_info(std::slice::from_ref(buffer_ref)),
            )
        }
        unsafe { device.update_descriptor_sets(&writes, &[]) };
    }
    
    pub fn clear(&mut self) {
        self.image_infos.clear();
        self.buffer_infos.clear();
        self.pending_buffer_writes.clear();
        self.pending_image_writes.clear();
    }
}

#[derive(Clone)]
pub struct DescriptorAllocator {
    device: Arc<VkDevice>,
    ratios: Vec<PoolSizeRatio>,
    pub full_pools: Vec<DescriptorPool>,
    pub ready_pools: Vec<DescriptorPool>,
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

    pub fn write_image_descriptors(
        &mut self,
        image_view: &ImageView,
        image_layout: &ImageLayout,
        shader_stage: ShaderStageFlags,
        descriptor_type: DescriptorType,
        sampler: Option<VkSampler>,
    ) -> Result<DescriptorSetDetails, Error> {
        let mut writer = DescriptorWriter::new();
        let mut descriptor_layout_builder = DescriptorLayoutBuilder::new();
        descriptor_layout_builder.add_binding(0, descriptor_type, shader_stage);
        let layout = descriptor_layout_builder.build(
            self.device.clone(),
            shader_stage,
            DescriptorSetLayoutCreateFlags::empty(),
        );

        let descriptor_set = self.allocate(self.device.clone(), &[layout]);
        writer.write_image(
            0,
            *image_view,
            sampler,
            *image_layout,
            descriptor_type,
        );
        writer.update_set(self.device.clone(), descriptor_set[0]);
        Ok(DescriptorSetDetails {
            descriptor_set: descriptor_set.to_vec(),
            layout: vec![layout],
        })
    }

    pub fn write_buffer_descriptors(
        &mut self,
        buffer: &VkBuffer,
        size: u64,
        shader_stage: ShaderStageFlags,
        descriptor_type: DescriptorType,
    ) -> Result<DescriptorSetDetails, Error> {
        let mut writer = DescriptorWriter::new();
        debug!("{size:?}");
        let mut descriptor_layout_builder = DescriptorLayoutBuilder::new();
        descriptor_layout_builder.add_binding(0, descriptor_type, shader_stage);
        let layout = descriptor_layout_builder.build(
            self.device.clone(),
            shader_stage,
            DescriptorSetLayoutCreateFlags::empty(),
        );

        let descriptor_set = self.allocate(self.device.clone(), &[layout]);
        writer.write_buffer(0, *buffer, size, 0, descriptor_type);
        writer.update_set(self.device.clone(), descriptor_set[0]);
        Ok(DescriptorSetDetails {
            descriptor_set: descriptor_set.to_vec(),
            layout: vec![layout],
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
            self.clear_pools();
        }
    }

    pub fn clear_pools(&mut self) {
        self.full_pools.clear();
        self.ready_pools.clear();
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

    pub fn allocate(
        &mut self,
        device: Arc<VkDevice>,
        layouts: &[DescriptorSetLayout],
    ) -> DescriptorSetDetails {
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
        DescriptorSetDetails {
            descriptor_set: descriptor_sets,
                layout: layouts.to_vec()
        }
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

    pub fn add_binding(&mut self, binding: u32, descriptor_type: DescriptorType, shader_stages: ShaderStageFlags) -> &mut Self {
        let descriptor_binding = DescriptorSetLayoutBinding::default()
            .binding(binding)
            .stage_flags(shader_stages)
            .descriptor_type(descriptor_type)
            .descriptor_count(1);

        self.bindings.push(descriptor_binding);
        self
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
