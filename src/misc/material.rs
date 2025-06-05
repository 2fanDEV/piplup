use std::sync::Arc;

use anyhow::Result;
use ash::vk::{
    ColorComponentFlags, CullModeFlags, DescriptorSetLayout, DescriptorSetLayoutCreateFlags,
    DescriptorType, DynamicState, Extent2D, FrontFace, ImageLayout, PipelineLayout,
    PolygonMode, PrimitiveTopology, SampleCountFlags, ShaderStageFlags,
};
use nalgebra::{Matrix4, Vector4};

use crate::{
    components::{
        allocation_types::{AllocatedImage, VkBuffer},
        descriptors::{
            DescriptorAllocator, DescriptorLayoutBuilder, DescriptorSetDetails, DescriptorWriter,
        },
        device::VkDevice,
        pipeline::{
            additive_blending, create_color_blending_attachment_state, create_multisampling_state,
            create_rasterizer_state, ShaderInformation, VkPipeline,
        },
        render_pass::VkRenderPass,
        sampler::VkSampler,
    },
    geom::push_constants::PushConstant,
};

#[derive(Clone, Debug, Default)]
pub struct MaterialPipeline {
    pub pipeline: VkPipeline,
    pub pipeline_layout: PipelineLayout,
}

#[derive(Default, Debug, Clone)]
pub struct MaterialConstants {
    pub color_factors: Vector4<f32>,
    pub metal_rough_factors: Vector4<f32>,
    //padding
    extra: Vector4<f32>,
}

impl MaterialConstants {
    pub fn new(
        color_factors: Vector4<f32>,
        metal_rough_factors: Vector4<f32>,
    ) -> MaterialConstants {
        Self {
            color_factors,
            metal_rough_factors,
            ..Default::default()
        }
    }
}

pub struct MaterialResources {
    pub color_image: AllocatedImage,
    pub color_sampler: VkSampler,
    pub metal_rough_image: AllocatedImage,
    pub metal_rough_sampler: VkSampler,
    pub data_buffer: VkBuffer,
    pub buffer_offset: u64,
}

#[derive(Default, Debug, Clone)]
pub struct MaterialInstance {
    pipeline: MaterialPipeline,
    material_set: DescriptorSetDetails,
    pass: MaterialPass,
}

#[allow(warnings)]
#[derive(Eq, PartialEq, PartialOrd, Ord, Debug, Clone, Default)]
pub enum MaterialPass {
    #[default]
    GLTF_PBR_MAIN_COLOR,
    GLTF_PBR_OPAQUE,
    GLTF_PBR_TRANSPARENT,
}

pub struct MaterialMetallicRoughness {
    opaque_pipeline: MaterialPipeline,
    transparent_pipeline: MaterialPipeline,
    material_layout: DescriptorSetLayout,
    writer: DescriptorWriter,
}

impl MaterialMetallicRoughness {
    pub fn build_pipelines(
        device: Arc<VkDevice>,
        extent: &Extent2D,
        render_pass: Arc<VkRenderPass>,
    ) -> Result<MaterialMetallicRoughness> {
        // TODO adjust path
        let shader_modules = [
            ShaderInformation::vertex_2d_information(
                "/Users/zapzap/Projects/piplup/shaders/scene_data_mesh.vert.spv".to_string(),
            ),
            ShaderInformation::fragment_2d_information(
                "/Users/zapzap/Projects/piplup/shaders/scene_data_mesh.frag.spv".to_string(),
            ),
        ];

        let mut layout_builder = DescriptorLayoutBuilder::new();
        layout_builder.add_binding(0, DescriptorType::UNIFORM_BUFFER, ShaderStageFlags::VERTEX);
        layout_builder.add_binding(
            1,
            DescriptorType::COMBINED_IMAGE_SAMPLER,
            ShaderStageFlags::FRAGMENT,
        );
        layout_builder.add_binding(
            2,
            DescriptorType::COMBINED_IMAGE_SAMPLER,
            ShaderStageFlags::FRAGMENT,
        );

        let layout = layout_builder.build(
            device.clone(),
            ShaderStageFlags::empty(),
            DescriptorSetLayoutCreateFlags::empty(),
        );

        let opaque_pipeline = VkPipeline::create_new_pipeline(
            device.clone(),
            &[DynamicState::SCISSOR, DynamicState::VIEWPORT],
            PrimitiveTopology::TRIANGLE_LIST,
            ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT,
            &shader_modules,
            Some(&[
                DescriptorLayoutBuilder::new()
                    .add_binding(
                        0,
                        DescriptorType::UNIFORM_BUFFER,
                        ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT,
                    )
                    .build(
                        device.clone(),
                        ShaderStageFlags::empty(), // Not actually used by any binding here, just for consistency
                        DescriptorSetLayoutCreateFlags::empty(),
                    ),
                layout,
            ]),
            extent,
            Some(PushConstant::<Matrix4<f32>>::default()),
            vec![],
            vec![],
            &[create_color_blending_attachment_state(
                ColorComponentFlags::R
                    | ColorComponentFlags::G
                    | ColorComponentFlags::B
                    | ColorComponentFlags::A,
                false,
                None,
                None,
                None,
                None,
                None,
                None,
            )],
            create_rasterizer_state(PolygonMode::FILL, CullModeFlags::NONE, FrontFace::CLOCKWISE),
            create_multisampling_state(false, SampleCountFlags::TYPE_1, 1.0, false, false),
            render_pass.clone(),
            true,
        )?;
        let transparent_pipeline = VkPipeline::create_new_pipeline(
            device.clone(),
            &[DynamicState::SCISSOR, DynamicState::VIEWPORT],
            PrimitiveTopology::TRIANGLE_LIST,
            ShaderStageFlags::VERTEX,
            &shader_modules,
            Some(&[
                DescriptorLayoutBuilder::new()
                    .add_binding(
                        0,
                        DescriptorType::UNIFORM_BUFFER,
                        ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT,
                    )
                    .build(
                        device.clone(),
                        ShaderStageFlags::empty(), // Not actually used by any binding here, just for consistency
                        DescriptorSetLayoutCreateFlags::empty(),
                    ),
                layout,
            ]),
            &Extent2D::default(),
            Some(PushConstant::<Matrix4<f32>>::default()),
            vec![],
            vec![],
            &[additive_blending()],
            create_rasterizer_state(PolygonMode::FILL, CullModeFlags::NONE, FrontFace::CLOCKWISE),
            create_multisampling_state(false, SampleCountFlags::TYPE_1, 1.0, false, false),
            render_pass.clone(),
            false,
        )?;

        Ok(Self {
            opaque_pipeline: MaterialPipeline {
                pipeline: opaque_pipeline,
                pipeline_layout: opaque_pipeline.pipeline_layout,
            },
            transparent_pipeline: MaterialPipeline {
                pipeline_layout: transparent_pipeline.pipeline_layout,
                pipeline: transparent_pipeline,
            },
            material_layout: layout,
            writer: DescriptorWriter::new(),
        })
    }

    pub fn write_material(
        mut self,
        device: Arc<VkDevice>,
        material_pass: MaterialPass,
        resources: MaterialResources,
        descriptor_allocator: &mut DescriptorAllocator,
    ) -> Result<MaterialInstance> {
        let mut pipeline = self.opaque_pipeline;
        if material_pass.eq(&MaterialPass::GLTF_PBR_TRANSPARENT) {
            pipeline = self.transparent_pipeline;
        }
        let descriptor_set = descriptor_allocator.allocate(device.clone(), &[self.material_layout]);
        self.writer.clear();
        self.writer.write_buffer(
            0,
            resources.data_buffer,
            size_of::<MaterialConstants>() as u64,
            resources.buffer_offset,
            DescriptorType::UNIFORM_BUFFER,
        );
        self.writer.write_image(
            1,
            resources.color_image.image_details.image_view,
            Some(resources.color_sampler),
            ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            DescriptorType::COMBINED_IMAGE_SAMPLER,
        );
        self.writer.write_image(
            2,
            resources.metal_rough_image.image_details.image_view,
            Some(resources.metal_rough_sampler),
            ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            DescriptorType::COMBINED_IMAGE_SAMPLER,
        );
        self.writer.update_set(device.clone(), descriptor_set[0]);
        Ok(MaterialInstance {
            pipeline: pipeline,
            material_set: descriptor_set,
            pass: material_pass,
        })
    }
}
