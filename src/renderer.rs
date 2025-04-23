use std::{borrow::Cow, collections::HashMap, io::Error, ops::Add, sync::Arc};

use ash::{
    ext::debug_utils,
    vk::{
        ClearValue, CommandBuffer, CommandBufferBeginInfo, CommandBufferResetFlags,
        CommandBufferUsageFlags, DebugUtilsMessengerEXT, DescriptorSetLayout, DescriptorType,
        Extent2D, Fence, ImageCreateInfo, ImageLayout, IndexType, Offset2D, PipelineBindPoint,
        PipelineStageFlags, PresentInfoKHR, Queue, Rect2D, RenderPassBeginInfo, Semaphore,
        ShaderStageFlags, SubmitInfo, SubpassContents, Viewport,
    },
};
use cgmath::Matrix4;
use egui::{Color32, FullOutput, ImageData, ImageSource, RichText, TextureId, WidgetText};
use log::{debug, error};
use vk_mem::AllocatorCreateInfo;
use winit::window::Window;

const MAX_FRAMES: usize = 2;

use crate::{
    components::{
        allocated_image::AllocatedImage,
        buffers::VkFrameBuffer,
        command_buffers::VkCommandPool,
        descriptors::{self, DescriptorAllocator, DescriptorSetDetails, PoolSizeRatio},
        device::{self, VkDevice},
        frame_data::FrameData,
        image_util::image_transition,
        instance::{self, VkInstance},
        memory_allocator::MemoryAllocator,
        pipeline::{ShaderInformation, VkPipeline},
        queue::{QueueType, VkQueue},
        render_pass::VkRenderPass,
        sampler::{self, VkSampler},
        surface,
        swapchain::{ImageDetails, KHRSwapchain},
    },
    egui::{
        image_information_data::TextureInformationData,
        integration::{EguiIntegration, MeshBuffers},
    },
};

#[allow(unused)]
pub struct Renderer {
    instance: Arc<VkInstance>,
    debug_instance: debug_utils::Instance,
    debugger: DebugUtilsMessengerEXT,
    device: Arc<VkDevice>,
    graphics_queue: Arc<VkQueue>,
    presentation_queue: Arc<VkQueue>,
    swapchain: Arc<KHRSwapchain>,
    render_pass: Arc<VkRenderPass>,
    memory_allocator: Arc<MemoryAllocator>,
    graphics_pipeline: Vec<VkPipeline>,
    egui_sampler: VkSampler,
    egui_descriptor_allocator: DescriptorAllocator,
    egui_descriptor_set_details: DescriptorSetDetails,
    allocated_image: AllocatedImage,
    font_id: TextureId,
    font_image: AllocatedImage,
    image_details: Vec<ImageDetails>,
    viewports: Vec<Viewport>,
    scissors: Vec<Rect2D>,
    framebuffers: Vec<VkFrameBuffer>,
    frame_data: Vec<FrameData>,
    frame_idx: usize,
    render_area: Rect2D,
    extent: Extent2D,
    pub integration: EguiIntegration,
    mesh_buffers: Vec<MeshBuffers>,
    command_pool: VkCommandPool,
}

#[allow(unused)]
struct ImageIndex {
    index: u32,
    recreate_swapchain: bool,
}

impl ImageIndex {
    pub fn new(input: (u32, bool)) -> Self {
        Self {
            index: input.0,
            recreate_swapchain: input.1,
        }
    }
}

impl Renderer {
    pub fn init(window: &Window) -> Result<Renderer, Error> {
        let vk_instance = Arc::new(instance::VkInstance::new(window)?);
        let (debug_instance, debugger) = instance::VkInstance::create_debugger(vk_instance.clone());
        let surface = Arc::new(surface::KHRSurface::new(vk_instance.clone(), window)?);
        let vk_device = Arc::new(device::VkDevice::new(
            vk_instance.clone(),
            surface.clone(),
            window,
        )?);
        let graphics_queue = Arc::new(VkQueue::new(
            vk_device.clone(),
            surface.clone(),
            QueueType::GRAPHICS_QUEUE,
        )?);
        let presentation_queue = Arc::new(VkQueue::new(
            vk_device.clone(),
            surface.clone(),
            QueueType::PRESENT_QUEUE,
        )?);
        let swapchain = Arc::new(KHRSwapchain::new(
            vk_instance.clone(),
            vk_device.clone(),
            surface.clone(),
            window,
            [graphics_queue.clone(), presentation_queue.clone()],
        )?);

        let extent = swapchain.details.clone().choose_swapchain_extent(window);
        let memory_allocator = Arc::new(MemoryAllocator::new(
            vk_device.clone(),
            AllocatorCreateInfo::new(&vk_instance, &vk_device, vk_device.physical_device),
        ));
        let image_details = swapchain.create_image_details()?;
        let allocated_image = memory_allocator.create_image(swapchain.clone())?;
        /*       let compute_descriptor_allocator = DescriptorAllocator::new(
                    vk_device.clone(),
                    10,
                    vec![PoolSizeRatio::new(DescriptorType::STORAGE_IMAGE, 1.0)],
                );
                let compute_descriptor_set_details = compute_descriptor_allocator.get_descriptors(
                    allocated_image.image_view,
                    ShaderStageFlags::COMPUTE,
                    DescriptorType::STORAGE_IMAGE,
                    None,
                )?;

                let compute_pipelines = VkPipeline::compute_pipelines(
                    vk_device.clone(),
                    &[compute_descriptor_set_details.layout],
                    "shaders/compute_shader.spv",
                )?;

        */
        let render_pass = Arc::new(VkRenderPass::new(
            vk_device.clone(),
            swapchain.details.clone().choose_swapchain_format().format,
        )?);
        let framebuffers = VkFrameBuffer::create_framebuffers(
            vk_device.clone(),
            render_pass.clone(),
            extent,
            &image_details,
        );
        let mut frame_data: Vec<FrameData> = Vec::new();
        let command_pool = VkCommandPool::new(graphics_queue.clone());
        for _i in 0..MAX_FRAMES {
            frame_data.push(FrameData::new(vk_device.clone(), &command_pool));
        }

        let egui_sampler = VkSampler::get_font_sampler(vk_device.clone());
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
        while let full_output = integration.run(
            |ctx| {
                egui::Window::new(WidgetText::default().strong())
                    .open(&mut true)
                    .vscroll(false)
                    .resizable(true)
                    .default_size([1200.0, 1200.0])
                    .show(&ctx, |ui| {
                        ui.label("Hello world!");
                        ui.spacing();
                        ui.spinner();
                        if ui.button("Click me").clicked() {
                            debug!("CLICKED");
                        }
                        if ui.button("WHAT THE HEEEEEEELLL").clicked() {
                            debug!("WHAT THE HEEEEELL");
                        }
                    });
            },
            window,
        ) {
            let textures_delta_set = full_output.textures_delta.set;
            if textures_delta_set.len() == 0 {
                break;
            }
            for delta in textures_delta_set {
                texture_informations.insert(
                    delta.0,
                    TextureInformationData::new(
                        full_output,
                        delta,
                        |image_data| {
                            memory_allocator
                                .create_texture_image(&[graphics_queue], &command_pool, &image_data)
                                .unwrap()
                        },
                        |allocated_image| {
                            egui_descriptor_allocator
                                .get_descriptors(
                                    &allocated_image.image_view,
                                    ShaderStageFlags::FRAGMENT,
                                    DescriptorType::COMBINED_IMAGE_SAMPLER,
                                    Some(egui_sampler),
                                )
                                .unwrap()
                        },
                    ),
                );
            }
        }

        let graphics_pipeline = VkPipeline::egui_pipeline(
            vk_device.clone(),
            &[
                ShaderInformation::vertex_2d_information(
                    "/Users/zapzap/Projects/piplup/shaders/2D_vertex_shader.spv".to_string(),
                ),
                ShaderInformation::fragment_2d_information(
                    "/Users/zapzap/Projects/piplup/shaders/2D_fragment_shader.spv".to_string(),
                ),
            ],
            &texture_informations
                .iter()
                .map(|entry| entry.1.descriptor_set_details.layout)
                .collect::<Vec<DescriptorSetLayout>>(),
            &extent,
            render_pass.clone(),
        )?;

        let mesh_buffers: Vec<MeshBuffers> = integration
            .convert(extent, full_output)
            .into_iter()
            .map(|mesh| {
                MeshBuffers::new(
                    mesh,
                    &memory_allocator,
                    graphics_queue.clone(),
                    &command_pool,
                )
                .unwrap()
            })
            .collect();
        let render_area = Rect2D::default()
            .offset(Offset2D::default().y(0).x(0))
            .extent(extent.clone());
        let viewports = vec![Viewport::default()
            .x(0.0)
            .y(0.0)
            .width(extent.width as f32)
            .height(extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)];
        let scissors = vec![Rect2D::default()
            .offset(Offset2D::default().x(0).y(0))
            .extent(extent)];
        Ok(Self {
            instance: vk_instance,
            debug_instance,
            debugger,
            device: vk_device,
            graphics_queue,
            presentation_queue,
            swapchain,
            //    compute_pipelines,
            //   compute_descriptor_set_details,
            //  compute_descriptor_allocator,
            egui_descriptor_allocator,
            egui_descriptor_set_details: egui_descriptor_set_details_font,
            graphics_pipeline,
            render_pass,
            allocated_image,
            font_id: font_id.0,
            font_image,
            framebuffers,
            image_details,
            egui_sampler,
            memory_allocator,
            frame_data,
            frame_idx: 0,
            render_area,
            integration,
            mesh_buffers,
            command_pool,
            viewports,
            scissors,
            extent,
        })
    }

    pub fn display(&mut self, window: &Window) {
        let frame_data = self.frame_data[self.frame_idx].clone();
        self.draw(&frame_data, window);
        self.frame_idx = self.frame_idx.add(1 as usize) % MAX_FRAMES;
    }

    fn draw(&mut self, frame_data: &FrameData, window: &Window) {
        unsafe {
            self.device
                .wait_for_fences(&frame_data.render_fence, true, u64::MAX)
                .unwrap();
            self.device.reset_fences(&frame_data.render_fence).unwrap();

            let image_index = ImageIndex::new(
                self.swapchain
                    .s_device
                    .acquire_next_image(
                        **self.swapchain,
                        u64::MAX,
                        frame_data.swapchain_semaphore[0],
                        Fence::null(),
                    )
                    .unwrap(),
            );

            self.device
                .reset_command_buffer(frame_data.command_buffer, CommandBufferResetFlags::empty())
                .unwrap();

            let full_output = self.integration.run(
                |ctx| {
                    egui::Window::new(WidgetText::default().strong())
                        .open(&mut true)
                        .vscroll(true)
                        .resizable(true)
                        .show(&ctx, |ui| {
                            ui.label("Hello world!");
                            if ui.button("Click me").clicked() {
                                debug!("CLICKED");
                            }
                            ui.image(egui::include_image!(
                                "/Users/zapzap/Projects/piplup/shaders/ferris.svg"
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
                .convert(self.extent, full_output)
                .into_iter()
                .map(|mesh| {
                    MeshBuffers::new(
                        mesh,
                        &self.memory_allocator,
                        self.graphics_queue.clone(),
                        &self.command_pool,
                    )
                    .unwrap()
                })
                .collect();
            self.record_command_buffer(frame_data, &image_index, &self.mesh_buffers, window);
            let stage_masks = vec![
                PipelineStageFlags::VERTEX_SHADER,
                PipelineStageFlags::FRAGMENT_SHADER,
            ];
            self.submit_queue(**self.graphics_queue, frame_data, &stage_masks);
            let image_indices = vec![image_index.index];
            self.present_queue(
                **self.graphics_queue,
                &frame_data.render_semaphore,
                &image_indices,
            );
        }
    }

    #[allow(dead_code)]
    fn immediate_submit<F: FnOnce(&Renderer, CommandBuffer)>(&self, function: F) {
        let command = self.command_pool.single_time_command().unwrap();
        function(self, command);
        self.command_pool
            .end_single_time_command(self.graphics_queue.clone(), command);
    }

    fn record_command_buffer(
        &self,
        frame_data: &FrameData,
        image_index: &ImageIndex,
        mesh_buffers: &[MeshBuffers],
        window: &Window,
    ) {
        unsafe {
            let begin_info =
                CommandBufferBeginInfo::default().flags(CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device
                .begin_command_buffer(frame_data.command_buffer, &begin_info)
                .unwrap();

            let clear_value = vec![ClearValue {
                color: ash::vk::ClearColorValue {
                    float32: [0.0, 0.0, 1.0, 1.0],
                },
            }];
            self.device.cmd_bind_pipeline(
                frame_data.command_buffer,
                PipelineBindPoint::GRAPHICS,
                *self.graphics_pipeline[0],
            );

            let render_pass_begin_info = RenderPassBeginInfo::default()
                .render_pass(**self.render_pass)
                .framebuffer(*self.framebuffers[image_index.index as usize])
                .clear_values(&clear_value)
                .render_area(self.render_area);

            self.device.cmd_begin_render_pass(
                frame_data.command_buffer,
                &render_pass_begin_info,
                SubpassContents::INLINE,
            );

            self.device
                .cmd_set_viewport(frame_data.command_buffer, 0, &self.viewports);
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
                frame_data.command_buffer,
                self.graphics_pipeline[0].pipeline_layout,
                ShaderStageFlags::VERTEX,
                0,
                matrix_bytes,
            );

            for mesh_buffer in mesh_buffers {
                debug!("{:?}", mesh_buffer.texture_id);
                if mesh_buffer.texture_id == self.font_id {
                    self.device.cmd_bind_descriptor_sets(
                        frame_data.command_buffer,
                        PipelineBindPoint::GRAPHICS,
                        self.graphics_pipeline[0].pipeline_layout,
                        0,
                        &[*self.egui_descriptor_set_details],
                        &[],
                    );
                }
                self.device
                    .cmd_set_scissor(frame_data.command_buffer, 0, &[mesh_buffer.scissors]);
                let vertex_buffer = vec![mesh_buffer.vertex_buffer.buffer];
                self.device.cmd_bind_vertex_buffers(
                    frame_data.command_buffer,
                    0,
                    &vertex_buffer,
                    &[0],
                );
                self.device.cmd_bind_index_buffer(
                    frame_data.command_buffer,
                    mesh_buffer.indices_buffer.buffer,
                    0,
                    IndexType::UINT32,
                );
                self.device.cmd_draw_indexed(
                    frame_data.command_buffer,
                    mesh_buffer.indices.len() as u32,
                    1,
                    0,
                    0,
                    0,
                );
            }

            self.device.cmd_end_render_pass(frame_data.command_buffer);
            self.device
                .end_command_buffer(frame_data.command_buffer)
                .unwrap();
        }
    }

    fn submit_queue(
        &self,
        queue: Queue,
        frame_data: &FrameData,
        stage_masks: &[PipelineStageFlags],
    ) {
        let command_buffers = frame_data.get_command_buffer();
        let submit_info = vec![SubmitInfo::default()
            .command_buffers(&command_buffers)
            .wait_dst_stage_mask(stage_masks)
            .signal_semaphores(&frame_data.render_semaphore)
            .wait_semaphores(&frame_data.swapchain_semaphore)];
        unsafe {
            self.device
                .queue_submit(queue, &submit_info, frame_data.render_fence[0])
                .unwrap()
        };
    }

    fn present_queue(&self, queue: Queue, wait_semaphores: &[Semaphore], image_indices: &[u32]) {
        let swapchains = vec![**self.swapchain];
        let present_info = PresentInfoKHR::default()
            .wait_semaphores(wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(image_indices);
        unsafe {
            self.swapchain
                .s_device
                .queue_present(queue, &present_info)
                .unwrap()
        };
    }
}
