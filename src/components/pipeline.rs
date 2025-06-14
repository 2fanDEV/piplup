use core::fmt::Debug;
use std::default;
use std::{io::Error, ops::Deref, sync::Arc};

use ash::vk::{
    BlendFactor, BlendOp, ComputePipelineCreateInfo, CullModeFlags, DescriptorSetLayout,
    DynamicState, Extent2D, FrontFace, GraphicsPipelineCreateInfo, LogicOp, Offset2D, Pipeline,
    PipelineCache, PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo,
    PipelineDynamicStateCreateInfo, PipelineInputAssemblyStateCreateInfo, PipelineLayout,
    PipelineLayoutCreateInfo, PipelineMultisampleStateCreateInfo,
    PipelineRasterizationStateCreateInfo, PipelineShaderStageCreateInfo,
    PipelineVertexInputStateCreateInfo, PipelineViewportStateCreateInfo, PolygonMode,
    PrimitiveTopology, PushConstantRange, Rect2D, SampleCountFlags, ShaderStageFlags,
    VertexInputAttributeDescription, VertexInputBindingDescription, Viewport,
};
use ash::vk::{ColorComponentFlags, CompareOp, PipelineDepthStencilStateCreateInfo};

use super::{device::VkDevice, render_pass::VkRenderPass, util::load_shader_module};

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct ShaderInformation {
    shader_file_path: String,
    stages: ShaderStageFlags,
    entry_point: String,
}

impl ShaderInformation {
    pub fn new(shader_file_path: String, stages: ShaderStageFlags, entry_point: String) -> Self {
        Self {
            shader_file_path,
            stages,
            entry_point,
        }
    }

    pub fn vertex_2d_information(shader_file_path: String) -> ShaderInformation {
        Self {
            shader_file_path,
            stages: ShaderStageFlags::VERTEX,
            entry_point: String::from("main"),
        }
    }

    pub fn fragment_2d_information(shader_file_path: String) -> ShaderInformation {
        Self {
            shader_file_path,
            stages: ShaderStageFlags::FRAGMENT,
            entry_point: String::from("main"),
        }
    }
}

#[derive(Clone, Copy, Default)]
pub enum PipelineType {
    #[default]
    GRAPHICS,
    COMPUTE,
}

#[allow(unused)]
#[derive(Clone, Copy, Default)]
pub struct VkPipeline {
    pipeline: Pipeline,
    pub pipeline_layout: PipelineLayout,
    pub pipeline_type: PipelineType,
}

impl Debug for VkPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pipeline")
            .field("Pipeline", &self.pipeline)
            .finish()
    }
}

impl Deref for VkPipeline {
    type Target = Pipeline;

    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl VkPipeline {
    pub fn create_new_pipeline<T>(
        device: Arc<VkDevice>,
        dynamic_state_list: &[DynamicState],
        topology: PrimitiveTopology,
        shader_stage_flags: ShaderStageFlags,
        shader_information: &[ShaderInformation],
        layouts: Option<&[DescriptorSetLayout]>,
        extent: &Extent2D,
        push_constant_range_type: Option<T>,
        vertex_binding_description: Vec<VertexInputBindingDescription>,
        vertex_attribute_description: Vec<VertexInputAttributeDescription>,
        color_attachment: &[PipelineColorBlendAttachmentState],
        rasterizer_info: PipelineRasterizationStateCreateInfo,
        multisampling_info: PipelineMultisampleStateCreateInfo,
        render_pass: Arc<VkRenderPass>,
        enable_depth_test: bool,
    ) -> Result<VkPipeline, Error> {
        let dynamic_states_create_info = dynamic_states(dynamic_state_list);
        let mut pipeline_stage_create_info: Vec<PipelineShaderStageCreateInfo> = Vec::new();
        for information in shader_information {
            let shader_module = load_shader_module(&information.shader_file_path, &device)?;
            pipeline_stage_create_info.push(
                PipelineShaderStageCreateInfo::default()
                    .name(c"main")
                    .module(shader_module)
                    .stage(information.stages),
            );
        }
        let vertex_input_state = PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&vertex_binding_description)
            .vertex_attribute_descriptions(&vertex_attribute_description);
        let input_assembly_state = PipelineInputAssemblyStateCreateInfo::default()
            .topology(topology)
            .primitive_restart_enable(false);
        let viewports = [create_viewport(extent)];
        let scissors = [create_scissor(extent)];
        let viewport_state = create_pipeline_viewport_state(&viewports, &scissors);
        let rasterizer_info = rasterizer_info;
        let multisamping_info = multisampling_info;
        let color_blending_attachments = color_attachment;

        let mut pipeline_layout_create_info = PipelineLayoutCreateInfo::default();

        let mut push_constant_range: Vec<PushConstantRange> = vec![];
        if push_constant_range_type.is_some() {
            push_constant_range.push(
                PushConstantRange::default()
                    .stage_flags(shader_stage_flags)
                    .size(size_of::<T>() as u32),
            );
            pipeline_layout_create_info =
                pipeline_layout_create_info.push_constant_ranges(&push_constant_range);
        }

        if layouts.is_some() {
            pipeline_layout_create_info = pipeline_layout_create_info.set_layouts(layouts.unwrap());
        }
        let pipeline_layout = unsafe {
            device
                .create_pipeline_layout(&pipeline_layout_create_info, None)
                .unwrap()
        };
        let color_blending_state_info = create_color_blending_state(color_blending_attachments);
        let depth_stencil_state_info = if enable_depth_test {
            enable_depth_stencil_state()
        } else {
            disable_depth_stencil_state()
        };
        let mut graphics_pipeline_create_info = GraphicsPipelineCreateInfo::default()
            .stages(&pipeline_stage_create_info)
            .dynamic_state(&dynamic_states_create_info)
            .input_assembly_state(&input_assembly_state)
            .vertex_input_state(&vertex_input_state)
            .viewport_state(&viewport_state)
            .color_blend_state(&color_blending_state_info)
            .multisample_state(&multisamping_info)
            .rasterization_state(&rasterizer_info)
            .layout(pipeline_layout)
            .render_pass(**render_pass)
            .base_pipeline_index(-1)
            .base_pipeline_handle(Pipeline::null())
            .depth_stencil_state(&depth_stencil_state_info);

        let pipeline = unsafe {
            &device
                .create_graphics_pipelines(
                    PipelineCache::default(),
                    &[graphics_pipeline_create_info],
                    None,
                )
                .unwrap()
                .into_iter()
                .map(|pipeline| Self {
                    pipeline,
                    pipeline_layout,
                    pipeline_type: PipelineType::GRAPHICS,
                })
                .collect::<Vec<VkPipeline>>()[0]
        };

        Ok(*pipeline)
    }

    pub fn compute_pipelines(
        device: Arc<VkDevice>,
        layouts: &[DescriptorSetLayout],
        shader_file_path: &str,
    ) -> Result<VkPipeline, Error> {
        let create_info = PipelineLayoutCreateInfo::default().set_layouts(layouts);
        let pipeline_layout = unsafe { device.create_pipeline_layout(&create_info, None).unwrap() };
        let shader_module = load_shader_module(shader_file_path, &device.device).unwrap();
        let shader_stage_info = PipelineShaderStageCreateInfo::default()
            .module(shader_module)
            .name(c"main")
            .stage(ShaderStageFlags::COMPUTE);

        let pipeline_create_info = vec![ComputePipelineCreateInfo::default()
            .stage(shader_stage_info)
            .layout(pipeline_layout)];
        let pipelines = unsafe {
            device.create_compute_pipelines(PipelineCache::null(), &pipeline_create_info, None)
        }
        .unwrap()
        .into_iter()
        .map(|pipeline| VkPipeline {
            pipeline,
            pipeline_layout,
            pipeline_type: PipelineType::COMPUTE,
        })
        .collect::<Vec<VkPipeline>>()[0];

        Ok(pipelines)
    }
}


fn disable_depth_stencil_state<'a>() -> PipelineDepthStencilStateCreateInfo<'a> {
    PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(false)
        .depth_write_enable(false)
        .stencil_test_enable(false)
        .depth_bounds_test_enable(false)
        .depth_compare_op(CompareOp::LESS)
        .min_depth_bounds(0.0)
        .max_depth_bounds(1.0)

}

fn enable_depth_stencil_state<'a>() -> PipelineDepthStencilStateCreateInfo<'a> {
    PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(true)
        .depth_write_enable(true)
        .stencil_test_enable(false)
        .depth_bounds_test_enable(false)
        .depth_compare_op(CompareOp::LESS)
        .min_depth_bounds(0.0)
        .max_depth_bounds(1.0)
}

pub fn dynamic_states(states: &[DynamicState]) -> PipelineDynamicStateCreateInfo<'_> {
    PipelineDynamicStateCreateInfo::default().dynamic_states(states)
}

pub fn create_viewport(extent: &Extent2D) -> Viewport {
    Viewport::default()
        .x(0.0)
        .y(0.0)
        .width(extent.width as f32)
        .height(extent.height as f32)
        .min_depth(0.0)
        .max_depth(1.0)
}

pub fn create_scissor(extent: &Extent2D) -> Rect2D {
    Rect2D::default()
        .offset(Offset2D::default().x(0).y(0))
        .extent(*extent)
}

pub fn create_pipeline_viewport_state<'a>(
    viewports: &'a [Viewport],
    scissors: &'a [Rect2D],
) -> PipelineViewportStateCreateInfo<'a> {
    PipelineViewportStateCreateInfo::default()
        .scissors(scissors)
        .viewports(viewports)
}

pub fn create_rasterizer_state<'a>(
    polygon_mode: PolygonMode,
    cull_mode: CullModeFlags,
    front_face: FrontFace,
) -> PipelineRasterizationStateCreateInfo<'a> {
    PipelineRasterizationStateCreateInfo::default()
        .depth_bias_enable(false)
        .rasterizer_discard_enable(false)
        .line_width(1.0)
        .polygon_mode(polygon_mode)
        .cull_mode(cull_mode)
        .front_face(front_face)
        .depth_bias_constant_factor(0.0)
        .depth_bias_slope_factor(0.0)
        .depth_bias_clamp(0.0)
}

pub fn create_multisampling_state<'a>(
    samping_shading_enabled: bool,
    rasterization_sampls: SampleCountFlags,
    min_simple_shading: f32,
    alpha_to_one_enable: bool,
    alpha_to_coverage_enable: bool,
) -> PipelineMultisampleStateCreateInfo<'a> {
    PipelineMultisampleStateCreateInfo::default()
        .sample_shading_enable(samping_shading_enabled)
        .rasterization_samples(rasterization_sampls)
        .min_sample_shading(min_simple_shading)
        .alpha_to_one_enable(alpha_to_one_enable)
        .alpha_to_coverage_enable(alpha_to_coverage_enable)
}

pub fn create_color_blending_attachment_state(
    color_write_mask: ColorComponentFlags,
    blend_enable: bool,
    src_color_blend_factor: Option<BlendFactor>,
    dst_color_blend_factor: Option<BlendFactor>,
    color_blend_op: Option<BlendOp>,
    src_alpha_blend_factor: Option<BlendFactor>,
    dst_alpha_blend_factor: Option<BlendFactor>,
    alpha_blend_op: Option<BlendOp>,
) -> PipelineColorBlendAttachmentState {
    if !blend_enable {
        return PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(color_write_mask);
    }
    PipelineColorBlendAttachmentState::default()
        .color_write_mask(color_write_mask)
        .blend_enable(blend_enable)
        .src_color_blend_factor(src_color_blend_factor.unwrap())
        .dst_color_blend_factor(dst_color_blend_factor.unwrap())
        .color_blend_op(color_blend_op.unwrap())
        .src_alpha_blend_factor(src_alpha_blend_factor.unwrap())
        .dst_alpha_blend_factor(dst_alpha_blend_factor.unwrap())
        .alpha_blend_op(alpha_blend_op.unwrap())
}

pub fn additive_blending() -> PipelineColorBlendAttachmentState {
    create_color_blending_attachment_state(
        ColorComponentFlags::R
            | ColorComponentFlags::G
            | ColorComponentFlags::B
            | ColorComponentFlags::A,
        true,
        Some(BlendFactor::SRC_ALPHA),
        Some(BlendFactor::ONE),
        Some(BlendOp::ADD),
        Some(BlendFactor::ONE),
        Some(BlendFactor::ZERO),
        Some(BlendOp::ADD),
    )
}

pub fn alpha_blending() -> PipelineColorBlendAttachmentState {
    create_color_blending_attachment_state(
        ColorComponentFlags::R
            | ColorComponentFlags::G
            | ColorComponentFlags::B
            | ColorComponentFlags::A,
        true,
        Some(BlendFactor::SRC_ALPHA),
        Some(BlendFactor::ONE),
        Some(BlendOp::ADD),
        Some(BlendFactor::ONE),
        Some(BlendFactor::ZERO),
        Some(BlendOp::ADD),
    )
}
pub fn create_color_blending_state(
    attachments: &[PipelineColorBlendAttachmentState],
) -> PipelineColorBlendStateCreateInfo {
    PipelineColorBlendStateCreateInfo::default()
        .attachments(attachments)
        .logic_op(LogicOp::COPY)
        .logic_op_enable(false)
        .blend_constants([0.0, 0.0, 0.0, 0.0])
}
