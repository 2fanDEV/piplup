use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Result};
use ash::vk::{
    AttachmentLoadOp, BlendFactor, BlendOp, ClearValue, ColorComponentFlags, CommandBuffer,
    CommandBufferBeginInfo, CommandBufferResetFlags, CommandBufferUsageFlags, CullModeFlags,
    DescriptorType, DynamicState, Extent2D, Format, FrontFace, IndexType, PipelineBindPoint,
    PolygonMode, PrimitiveTopology, Rect2D, RenderPass, RenderPassBeginInfo, SampleCountFlags,
    ShaderStageFlags, SubpassContents, Viewport,
};
use cgmath::{Matrix4, SquareMatrix};
use egui::{epaint::Vertex, TextureId, WidgetText};
use image_information_data::TextureInformationData;
use integration::EguiIntegration;
use log::debug;
use thiserror::Error;
use winit::window::Window;

use crate::{
    components::{
        allocation_types::VkFrameBuffer,
        command_buffers::{self, VkCommandPool},
        descriptors::{DescriptorAllocator, PoolSizeRatio},
        device::VkDevice,
        memory_allocator::MemoryAllocator,
        pipeline::{
            self, create_multisampling_state, create_rasterizer_state, ShaderInformation,
            VkPipeline,
        },
        queue::VkQueue,
        render_pass::VkRenderPass,
        sampler::VkSampler,
    },
    geom::{mesh::MeshBuffers, VertexAttributes},
    renderer::ImageIndex,
};

pub mod image_information_data;
pub mod integration;

#[derive(Error, Debug)]
pub enum EguiRenderError {
    #[error("{0} is not managed yet by the renderer!")]
    NotManaged(String),
}

pub struct EguiRenderer {
    device: Arc<VkDevice>,
    font_sampler: VkSampler,
    texture_sampler: VkSampler,
    descriptor_allocator: DescriptorAllocator,
    texture_informations: HashMap<TextureId, TextureInformationData>,
    pub integration: EguiIntegration,
    mesh_buffers: Vec<MeshBuffers<Vertex, u32>>,
    memory_allocator: Arc<MemoryAllocator>,
    graphics_queue: Arc<VkQueue>,
    command_pool: VkCommandPool,
    pipelines: Vec<VkPipeline>,
    render_pass: Arc<VkRenderPass>,
    extent: Extent2D,
}

impl EguiRenderer {
    pub fn new(
        vk_device: Arc<VkDevice>,
        window: &Window,
        memory_allocator: Arc<MemoryAllocator>,
        graphics_queue: Arc<VkQueue>,
        extent: Extent2D,
        format: Format,
    ) -> Result<Self> {
        let egui_cmd_pool: VkCommandPool =
            command_buffers::VkCommandPool::new(graphics_queue.clone());
        let egui_font_sampler = VkSampler::get_font_sampler(vk_device.clone());
        let egui_texture_sampler = VkSampler::get_texture_sampler(vk_device.clone());
        let egui_descriptor_allocator = DescriptorAllocator::new(
            vk_device.clone(),
            10,
            vec![PoolSizeRatio::new(
                DescriptorType::COMBINED_IMAGE_SAMPLER,
                1.0,
            )],
        );

        let mut texture_informations = HashMap::<TextureId, TextureInformationData>::new();
        let mut integration = EguiIntegration::new(window);
        let render_pass = Arc::new(VkRenderPass::new(
            vk_device.clone(),
            format,
            AttachmentLoadOp::LOAD,
        )?);
        #[allow(irrefutable_let_patterns)]
        while let full_output = integration.run(
            |ctx| {
                egui::Window::new(WidgetText::default().strong())
                    .open(&mut true)
                    .vscroll(true)
                    .resizable(true)
                    .show(ctx, |ui| {
                        ui.label("Hello world!");
                        if ui.button("Click me").clicked() {
                            debug!("CLICKED");
                        }
                        ui.image(egui::include_image!(
                            "/Users/zapzap/Projects/piplup/shaders/ferris.png"
                        ));
                        if ui.button("WTF").clicked() {
                            debug!("WTF");
                        }
                    });
            },
            window,
        ) {
            let textures_delta_set = full_output.textures_delta.set;
            if textures_delta_set.is_empty() {
                break;
            }

            for delta in textures_delta_set {
                debug!("DELTA: {:?}", delta.0);
                texture_informations.insert(
                    delta.0,
                    TextureInformationData::new(
                        delta.clone(),
                        |image_data| {
                            memory_allocator
                                .create_texture_image(
                                    &[graphics_queue.clone()],
                                    &egui_cmd_pool,
                                    image_data,
                                )
                                .unwrap()
                        },
                        |allocated_image| {
                            let sampler = match delta.0 {
                                TextureId::Managed(id) => match id {
                                    0 => Some(egui_font_sampler.clone()),
                                    _ => Some(egui_texture_sampler.clone()),
                                },
                                TextureId::User(_) => {
                                    return Err(anyhow!(EguiRenderError::NotManaged(
                                        String::from("User handled texture data",)
                                    )));
                                }
                            };
                            Ok(egui_descriptor_allocator
                                .get_descriptors(
                                    &allocated_image.image_view,
                                    ShaderStageFlags::FRAGMENT,
                                    DescriptorType::COMBINED_IMAGE_SAMPLER,
                                    sampler,
                                )
                                .unwrap())
                        },
                    ),
                );
            }
        }
        let egui_fragment_shader = vec![
            ShaderInformation::fragment_2d_information(
                "/Users/zapzap/Projects/piplup/shaders/2D_fragment_shader.spv".to_string(),
            ),
            ShaderInformation::fragment_2d_information(
                "/Users/zapzap/Projects/piplup/shaders/2D_texture_fragment_shader.spv".to_string(),
            ),
        ];
        let mut egui_pipelines: Vec<VkPipeline> = vec![];
        for shader in egui_fragment_shader {
            egui_pipelines.push(VkPipeline::create_new_pipeline(
                vk_device.clone(),
                &[DynamicState::SCISSOR, DynamicState::VIEWPORT],
                PrimitiveTopology::TRIANGLE_LIST,
                ShaderStageFlags::VERTEX,
                &[
                    ShaderInformation::vertex_2d_information(
                        "/Users/zapzap/Projects/piplup/shaders/2D_vertex_shader.spv".to_string(),
                    ),
                    shader,
                ],
                Some(&[texture_informations
                    .get(&TextureId::Managed(0))
                    .unwrap()
                    .descriptor_set_details
                    .layout]),
                &extent,
                Some(Matrix4::<f32>::identity()),
                Vertex::get_binding_description(),
                Vertex::get_attribute_description(),
                &[pipeline::create_color_blending_attachment_state(
                    ColorComponentFlags::R
                        | ColorComponentFlags::G
                        | ColorComponentFlags::B
                        | ColorComponentFlags::A,
                    true,
                    Some(BlendFactor::SRC_ALPHA),
                    Some(BlendFactor::ONE_MINUS_SRC_ALPHA),
                    Some(BlendOp::ADD),
                    Some(BlendFactor::SRC_ALPHA),
                    Some(BlendFactor::ONE_MINUS_SRC_ALPHA),
                    Some(BlendOp::ADD),
                )],
                create_rasterizer_state(
                    PolygonMode::FILL,
                    CullModeFlags::NONE,
                    FrontFace::CLOCKWISE,
                ),
                create_multisampling_state(false, SampleCountFlags::TYPE_1, 1.0, false, false),
                render_pass.clone(),
            )?);
        }
        Ok(Self {
            device: vk_device.clone(),
            font_sampler: egui_font_sampler,
            texture_sampler: egui_texture_sampler,
            descriptor_allocator: egui_descriptor_allocator,
            texture_informations,
            integration,
            extent,
            command_pool: egui_cmd_pool,
            pipelines: egui_pipelines,
            memory_allocator,
            render_pass,
            graphics_queue,
            mesh_buffers: vec![],
        })
    }

    pub fn draw(
        &mut self,
        command_buffer: CommandBuffer,
        image_index: &ImageIndex,
        window: &Window,
        viewports: Vec<Viewport>,
        render_area: Rect2D,
        framebuffers: &[VkFrameBuffer],
    ) -> Result<()> {
        let full_output = self.integration.run(
            |ctx| {
                egui::Window::new(WidgetText::default().strong())
                    .open(&mut true)
                    .vscroll(true)
                    .resizable(true)
                    .show(ctx, |ui| {
                        ui.label("Hello world!");
                        if ui.button("Click me").clicked() {
                            debug!("CLICKED");
                        }
                        ui.image(egui::include_image!(
                            "/Users/zapzap/Projects/piplup/shaders/ferris.png"
                        ));
                        if ui.button("WHAT THE HEEEEEEELLL").clicked() {
                            debug!("WHAT THE HEEEEELL");
                        }
                    });
            },
            window,
        );

        self.mesh_buffers = self
            .integration
            .convert(self.extent, &full_output)
            .into_iter()
            .map(|mesh| {
                MeshBuffers::<Vertex, u32>::new(
                    mesh,
                    |elements, flags, usage, mem_flags| {
                        self.memory_allocator.create_buffer(
                            &elements,
                            &[self.graphics_queue.clone()],
                            flags,
                            usage,
                            mem_flags,
                            &self.command_pool,
                        ).unwrap()
                    },
                    |elements, flags, usage, mem_flags| {
                        self.memory_allocator.create_buffer(
                            &elements,
                            &[self.graphics_queue.clone()],
                            flags,
                            usage,
                            mem_flags,
                            &self.command_pool,
                        ).unwrap()
                    },
                ).unwrap()
            })
            .collect();
        self.record_command_buffer(
            command_buffer,
            image_index,
            &self.mesh_buffers,
            framebuffers,
            **self.render_pass,
            render_area,
            viewports,
            window,
        )?;

        Ok(())
    }

    fn record_command_buffer(
        &self,
        command_buffer: CommandBuffer,
        image_index: &ImageIndex,
        mesh_buffers: &[MeshBuffers<Vertex, u32>],
        framebuffers: &[VkFrameBuffer],
        render_pass: RenderPass,
        render_area: Rect2D,
        viewports: Vec<Viewport>,
        window: &Window,
    ) -> Result<()> {
        unsafe {
            self.device
                .reset_command_buffer(command_buffer, CommandBufferResetFlags::empty())?;
            self.device.begin_command_buffer(
                command_buffer,
                &CommandBufferBeginInfo::default().flags(CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;
            let clear_value = vec![ClearValue {
                color: ash::vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            }];
            self.device.cmd_begin_render_pass(
                command_buffer,
                &RenderPassBeginInfo::default()
                    .clear_values(&clear_value)
                    .render_area(render_area)
                    .framebuffer(*framebuffers[image_index.index as usize])
                    .render_pass(render_pass),
                SubpassContents::INLINE,
            );
            self.device.cmd_set_viewport(command_buffer, 0, &viewports);
            let scale_factor = window.scale_factor();
            let logical_size = window.inner_size().to_logical::<f32>(scale_factor);

            let sx = 2.0 / logical_size.width;
            let sy = 2.0 / logical_size.height;
            let tx = -1.0;
            let ty = -1.0;

            let clip_matrix = Matrix4::new(
                sx, 0.0, 0.0, 0.0, 0.0, sy, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, tx, ty, 0.0, 1.0,
            );
            let matrix_array: &[[f32; 4]; 4] = clip_matrix.as_ref();

            // Get a pointer to the first element of the array
            let matrix_ptr: *const f32 = matrix_array.as_ptr() as *const f32;

            // Convert the pointer to a byte slice
            let matrix_bytes: &[u8] =
                std::slice::from_raw_parts(matrix_ptr as *const u8, size_of::<Matrix4<f32>>());

            self.device.cmd_push_constants(
                command_buffer,
                self.pipelines[0].pipeline_layout,
                ShaderStageFlags::VERTEX,
                0,
                matrix_bytes,
            );

            for mesh_buffer in mesh_buffers {
                let texture_information_data = self
                    .texture_informations
                    .get(&mesh_buffer.mesh.texture_id.unwrap());
                if texture_information_data.iter().len() > 0 {
                    match texture_information_data.unwrap().texture_id {
                        TextureId::Managed(id) => {
                            self.device.cmd_bind_pipeline(
                                command_buffer,
                                PipelineBindPoint::GRAPHICS,
                                *self.pipelines[id as usize],
                            );

                            self.device.cmd_bind_descriptor_sets(
                                command_buffer,
                                PipelineBindPoint::GRAPHICS,
                                self.pipelines[id as usize].pipeline_layout,
                                0,
                                &[*texture_information_data.unwrap().descriptor_set_details],
                                &[],
                            );
                        }
                        TextureId::User(_) => todo!(),
                    }
                }
                self.device
                    .cmd_set_scissor(command_buffer, 0, &[mesh_buffer.mesh.scissors]);
                let vertex_buffer = vec![mesh_buffer.vertex_buffer.buffer];
                self.device
                    .cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffer, &[0]);
                self.device.cmd_bind_index_buffer(
                    command_buffer,
                    mesh_buffer.index_buffer.buffer,
                    0,
                    IndexType::UINT32,
                );
                self.device.cmd_draw_indexed(
                    command_buffer,
                    mesh_buffer.mesh.indices.len() as u32,
                    1,
                    0,
                    0,
                    0,
                );
            }
            self.device.cmd_end_render_pass(command_buffer);
            self.device.end_command_buffer(command_buffer)?;
        }
        Ok(())
    }
}
