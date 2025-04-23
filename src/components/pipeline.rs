use std::{io::Error, ops::Deref, sync::Arc};

use ash::vk::ColorComponentFlags;
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
use cgmath::Matrix4;

use super::{
    device::VkDevice, geom::vertex::Vertex2D, render_pass::VkRenderPass, util::load_shader_module,
};

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct ShaderInformation {
    shader_file_path: String,
    stages: ShaderStageFlags,
    entry_point: String,
    vertex_binding_description: Vec<VertexInputBindingDescription>,
    vertex_attribute_description: Vec<VertexInputAttributeDescription>,
}

impl ShaderInformation {
    pub fn new(
        shader_file_path: String,
        stages: ShaderStageFlags,
        entry_point: String,
        vertex_binding_description: Vec<VertexInputBindingDescription>,
        vertex_attribute_description: Vec<VertexInputAttributeDescription>,
    ) -> Self {
        Self {
            shader_file_path,
            stages,
            entry_point,
            vertex_binding_description,
            vertex_attribute_description,
        }
    }

    pub fn vertex_2d_information(shader_file_path: String) -> ShaderInformation {
        Self {
            shader_file_path,
            stages: ShaderStageFlags::VERTEX,
            entry_point: String::from("main"),
            vertex_binding_description: Vertex2D::get_binding_description(),
            vertex_attribute_description: Vertex2D::get_attribute_description(),
        }
    }

    pub fn fragment_2d_information(shader_file_path: String) -> ShaderInformation {
        Self {
            shader_file_path,
            stages: ShaderStageFlags::FRAGMENT,
            entry_point: String::from("main"),
            vertex_binding_description: Vertex2D::get_binding_description(),
            vertex_attribute_description: Vertex2D::get_attribute_description(),
        }
    }
}

pub enum PipelineType {
    GRAPHICS,
    COMPUTE,
}

#[allow(unused)]
pub struct VkPipeline {
    pipeline: Pipeline,
    pub pipeline_layout: PipelineLayout,
    device: Arc<VkDevice>,
    pub pipeline_type: PipelineType,
}

impl Deref for VkPipeline {
    type Target = Pipeline;

    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl VkPipeline {
    pub fn compute_pipelines(
        device: Arc<VkDevice>,
        layouts: &[DescriptorSetLayout],
        shader_file_path: &str,
    ) -> Result<Vec<VkPipeline>, Error> {
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
            device: device.clone(),
            pipeline_type: PipelineType::COMPUTE,
        })
        .collect::<Vec<VkPipeline>>();

        Ok(pipelines)
    }

    pub fn egui_pipeline(
        device: Arc<VkDevice>,
        shader_information: &[ShaderInformation],
        layouts: &[DescriptorSetLayout],
        extent: &Extent2D,
        render_pass: Arc<VkRenderPass>,
    ) -> Result<Vec<VkPipeline>, Error> {
        let dynamic_states_create_info =
            dynamic_states(&[DynamicState::VIEWPORT, DynamicState::SCISSOR]);
        let mut pipeline_stage_create_info: Vec<PipelineShaderStageCreateInfo> = Vec::new();
        let vertex_binding_description: Vec<VertexInputBindingDescription> = Vertex2D::get_binding_description();
        let vertex_attribute_description: Vec<VertexInputAttributeDescription> = Vertex2D::get_attribute_description();
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
            .topology(PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);
        let viewports = [create_viewport(extent)];
        let scissors = [create_scissor(extent)];
        let viewport_state = create_pipeline_viewport_state(&viewports, &scissors);
        let rasterizer_info = create_rasterizer_state();
        let multisamping_info = create_multisampling_state();
        let color_blending_attachments = [create_color_blending_attachment_state()];

        let push_constant_ranges = vec![PushConstantRange::default()
            .stage_flags(ShaderStageFlags::VERTEX)
            .size(size_of::<Matrix4<f32>>() as u32)];
        let pipeline_layout_create_info =
            PipelineLayoutCreateInfo::default().push_constant_ranges(&push_constant_ranges)
            .set_layouts(layouts);
        let pipeline_layout = unsafe {
            device
                .create_pipeline_layout(&pipeline_layout_create_info, None)
                .unwrap()
        };
        let color_blending_state_info = create_color_blending_state(&color_blending_attachments);
        let graphics_pipeline_create_info = GraphicsPipelineCreateInfo::default()
            .stages(&pipeline_stage_create_info)
            .dynamic_state(&dynamic_states_create_info)
            .input_assembly_state(&input_assembly_state)
            .vertex_input_state(&vertex_input_state)
            .viewport_state(&viewport_state)
            .color_blend_state(&color_blending_state_info)
            .multisample_state(&multisamping_info)
            .rasterization_state(&rasterizer_info)
            .layout(pipeline_layout)
            .subpass(0)
            .render_pass(**render_pass)
            .base_pipeline_index(-1)
            .base_pipeline_handle(Pipeline::null());
        //        .depth_stencil_state(depth_stencil_state);
        let pipelines = unsafe {
            device
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
                    device: device.clone(),
                    pipeline_type: PipelineType::GRAPHICS,
                })
                .collect::<Vec<VkPipeline>>()
        };

        Ok(pipelines)
    }
}

fn dynamic_states<'a>(states: &'a [DynamicState]) -> PipelineDynamicStateCreateInfo<'a> {
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

fn create_pipeline_viewport_state<'a>(
    viewports: &'a [Viewport],
    scissors: &'a [Rect2D],
) -> PipelineViewportStateCreateInfo<'a> {
    PipelineViewportStateCreateInfo::default()
        .scissors(scissors)
        .viewports(viewports)
}

fn create_rasterizer_state<'a>() -> PipelineRasterizationStateCreateInfo<'a> {
    PipelineRasterizationStateCreateInfo::default()
        .depth_bias_enable(false)
        .rasterizer_discard_enable(false)
        .line_width(1.0)
        .polygon_mode(PolygonMode::FILL)
        .cull_mode(CullModeFlags::NONE)
        .front_face(FrontFace::CLOCKWISE)
        .depth_bias_constant_factor(0.0)
        .depth_bias_slope_factor(0.0)
        .depth_bias_clamp(0.0)
}

fn create_multisampling_state<'a>() -> PipelineMultisampleStateCreateInfo<'a> {
    PipelineMultisampleStateCreateInfo::default()
        .sample_shading_enable(false)
        .rasterization_samples(SampleCountFlags::TYPE_1)
        .min_sample_shading(1.0)
        .alpha_to_one_enable(false)
        .alpha_to_coverage_enable(false)
}

fn create_color_blending_attachment_state() -> PipelineColorBlendAttachmentState {
    PipelineColorBlendAttachmentState::default()
        .color_write_mask(
            ColorComponentFlags::R
                | ColorComponentFlags::G
                | ColorComponentFlags::B
                | ColorComponentFlags::A,
        )
        .blend_enable(true)
        .src_color_blend_factor(BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(BlendOp::ADD)
        .src_alpha_blend_factor(BlendFactor::ONE)
        .dst_color_blend_factor(BlendFactor::ONE_MINUS_SRC_ALPHA)
        .alpha_blend_op(BlendOp::ADD)
}

fn create_color_blending_state(
    attachments: &[PipelineColorBlendAttachmentState],
) -> PipelineColorBlendStateCreateInfo {
    PipelineColorBlendStateCreateInfo::default()
        .attachments(attachments)
        .logic_op(LogicOp::COPY)
        .logic_op_enable(false)
        .blend_constants([0.0, 0.0, 0.0, 0.0])
}
