use std::sync::Arc;

use anyhow::Result;
use ash::vk::{
    ColorComponentFlags, CullModeFlags, DescriptorSetLayout, DescriptorSetLayoutCreateFlags,
    DescriptorType, DynamicState, Extent2D, FrontFace, PipelineLayout,
    PipelineMultisampleStateCreateInfo, PolygonMode, PrimitiveTopology, PushConstantRange,
    RenderPass, ShaderModule, ShaderStageFlags,
};
use nalgebra::{Matrix4, Vector4};

use crate::{
    components::{
        allocation_types::{AllocatedImage, VkBuffer}, descriptors::{DescriptorAllocator, DescriptorLayoutBuilder, DescriptorSetDetails}, device::VkDevice, pipeline::{
            additive_blending, create_color_blending_attachment_state, create_rasterizer_state, ShaderInformation, VkPipeline
        }, render_pass::VkRenderPass, sampler::VkSampler
    },
    geom::push_constants::PushConstant,
    renderer::{self, Renderer},
};

pub struct MaterialPipeline {
    pub pipeline: VkPipeline,
    pub pipeline_layout: PipelineLayout,
}

struct MaterialConstants {
    color_factors: Vector4::<f32>,
    metal_rough_factors: Vector4::<f32>,
    //padding
    extra: Vector4<f32>
}

struct MaterialResources {
    color_image: AllocatedImage,
    color_sampler: VkSampler,
    metal_rough_image: AllocatedImage,
    metal_rough_sampler: VkSampler,
    data_buffer: VkBuffer,
    buffer_offset: u32
}

pub struct MaterialInstance {
    pipeline: MaterialPipeline,
    material_set: DescriptorSetDetails,
    pass: MaterialPass,
}

#[allow(warnings)]
#[derive(Eq, PartialEq, PartialOrd, Ord, Debug)]
pub enum MaterialPass {
    GLTF_PBR_MAIN_COLOR,
    GLTF_PBR_OPAQUE,
    GLTF_PBR_TRANSPARENT,
}

pub struct MaterialMetallicRoughness {
    opaque_pipeline: MaterialPipeline,
    transparent_pipeline: MaterialPipeline,
}

impl MaterialMetallicRoughness {
    fn build_pipelines(
        device: Arc<VkDevice>,
        render_pass: Arc<VkRenderPass>,
        layout: DescriptorSetLayout,
    ) -> Result<MaterialMetallicRoughness> {
        // TODO adjust path
        let shader_modules = [
            ShaderInformation::fragment_2d_information(
                "/Users/zapzap/Projects/piplup/shaders/scene_data_mesh.vert.spv".to_string(),
            ),
            ShaderInformation::fragment_2d_information(
                "/Users/zapzap/Projects/piplup/shaders/scene_data_mesh.frag.spv".to_string(),
            ),
        ];

        let mut layout_builder = DescriptorLayoutBuilder::new();
        layout_builder.add_binding(0, DescriptorType::UNIFORM_BUFFER);
        layout_builder.add_binding(1, DescriptorType::COMBINED_IMAGE_SAMPLER);
        layout_builder.add_binding(2, DescriptorType::COMBINED_IMAGE_SAMPLER);
        let layout = layout_builder.build(
            device.clone(),
            ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT,
            DescriptorSetLayoutCreateFlags::empty(),
        );
        let opaque_pipeline = VkPipeline::create_new_pipeline(
            device.clone(),
            &[],
            PrimitiveTopology::TRIANGLE_LIST,
            ShaderStageFlags::VERTEX,
            &shader_modules,
            Some(&[layout]),
            &Extent2D::default(),
            Some(PushConstant::<Matrix4<f32>>::default()),
            vec![],
            vec![],
            &[create_color_blending_attachment_state(
                ColorComponentFlags::empty(),
                false,
                None,
                None,
                None,
                None,
                None,
                None,
            )],
            create_rasterizer_state(PolygonMode::FILL, CullModeFlags::NONE, FrontFace::CLOCKWISE),
            PipelineMultisampleStateCreateInfo::default(),
            render_pass.clone(),
        )?;

        let transparent_pipeline = VkPipeline::create_new_pipeline(
            device.clone(),
            &[],
            PrimitiveTopology::TRIANGLE_LIST,
            ShaderStageFlags::VERTEX,
            &shader_modules,
            Some(&[layout]),
            &Extent2D::default(),
            Some(PushConstant::<Matrix4<f32>>::default()),
            vec![],
            vec![],
            &[additive_blending()],
            create_rasterizer_state(PolygonMode::FILL, CullModeFlags::NONE, FrontFace::CLOCKWISE),
            PipelineMultisampleStateCreateInfo::default(),
            render_pass.clone(),
            // TODO add enable depth test and disable above
        )?;

        Ok(Self { opaque_pipeline:  MaterialPipeline {
            pipeline: opaque_pipeline,
            pipeline_layout: opaque_pipeline.pipeline_layout
        },
        transparent_pipeline: MaterialPipeline {
            pipeline_layout: transparent_pipeline.pipeline_layout,
            pipeline: transparent_pipeline
        }}
        )
    }


    pub fn write_material(&self, device: VkDevice, material_pass: MaterialPass, resources: MaterialResources, descriptor_allocator: DescriptorAllocator) {
           let mut pipeline = &self.opaque_pipeline;
            if material_pass.eq(&MaterialPass::GLTF_PBR_TRANSPARENT) {
                pipeline = &self.transparent_pipeline;
            }

//            material_set = descriptor_allocator.write_image_descriptors(image_view, image_layout, shader_stage, descriptor_type, sampler)
    }
}
