use std::{io::Error, ops::Add, sync::Arc};

use ash::{
    ext::debug_utils,
    vk::{
        ClearValue, CommandBuffer, CommandBufferBeginInfo, CommandBufferResetFlags, CommandBufferUsageFlags, DebugUtilsMessengerEXT, DescriptorType, Extent2D, Fence, ImageLayout, IndexType, Offset2D, PipelineBindPoint, PipelineStageFlags, PresentInfoKHR, Queue, Rect2D, RenderPassBeginInfo, Sampler, Semaphore, ShaderStageFlags, SubmitInfo, SubpassContents, Viewport
    },
};
use cgmath::Matrix4;
use log::debug;
use vk_mem::{Allocator, AllocatorCreateInfo};
use winit::window::Window;

const MAX_FRAMES: usize = 2;

use crate::{
    components::{
        allocated_image::AllocatedImage, buffers::VkFrameBuffer, command_buffers::VkCommandPool, descriptors::{DescriptorAllocator, DescriptorSetDetails, PoolSizeRatio}, device::{self, VkDevice}, frame_data::FrameData, image_util, instance::{self, VkInstance}, memory_allocator::MemoryAllocator, pipeline::{ShaderInformation, VkPipeline}, queue::{QueueType, VkQueue}, render_pass::VkRenderPass, sampler::VkSampler, surface, swapchain::{ImageDetails, KHRSwapchain}
    },
    egui::integration::{EguiIntegration, MeshBuffers},
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
    compute_pipelines: Vec<VkPipeline>,
    render_pass: Arc<VkRenderPass>,
    memory_allocator: Arc<MemoryAllocator>,
    graphics_pipeline: Vec<VkPipeline>,
    compute_descriptor_allocator: DescriptorAllocator,
    compute_descriptor_set_details: DescriptorSetDetails,
    egui_sampler: VkSampler,
    egui_descriptor_allocator: DescriptorAllocator,
    egui_descriptor_set_details: DescriptorSetDetails,
    allocated_image: AllocatedImage,
    image_details: Vec<ImageDetails>,
    viewports: Vec<Viewport>,
    scissors: Vec<Rect2D>,
    framebuffers: Vec<VkFrameBuffer>,
    frame_data: Vec<FrameData>,
    frame_idx: usize,
    render_area: Rect2D,
    extent: Extent2D,
    pub integration: EguiIntegration,
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
        let memory_allocator = Arc::new( MemoryAllocator::new(AllocatorCreateInfo::new(
                &vk_instance,
                &vk_device,
                vk_device.physical_device,
            )));
        let image_details = swapchain.create_image_details()?;
        let allocated_image = memory_allocator.create_image(swapchain.clone())?;
        let fonts_image = memory_allocator.create_image(swapchain.clone())?;
        let compute_descriptor_allocator = DescriptorAllocator::new(
            vk_device.clone(),
            10,
            vec![PoolSizeRatio::new(DescriptorType::STORAGE_IMAGE, 1.0)],
        );
        let compute_descriptor_set_details = compute_descriptor_allocator.get_descriptors(
            allocated_image.image_view,
            ShaderStageFlags::COMPUTE,
            DescriptorType::STORAGE_IMAGE,
            None
        )?;

        let egui_sampler = VkSampler::get_font_sampler(vk_device.clone());

        let egui_descriptor_allocator = DescriptorAllocator::new(
            vk_device.clone(),
            10,
            vec![PoolSizeRatio::new(
                DescriptorType::COMBINED_IMAGE_SAMPLER,
                1.0,
            )],
        );
        let egui_descriptor_set_details = egui_descriptor_allocator.get_descriptors(
            fonts_image.image_view,
            ShaderStageFlags::FRAGMENT,
            DescriptorType::COMBINED_IMAGE_SAMPLER,
            Some(egui_sampler.clone())
        )?;

        let compute_pipelines = VkPipeline::compute_pipelines(
            vk_device.clone(),
            &[compute_descriptor_set_details.layout],
            "shaders/compute_shader.spv",
        )?;
        let render_pass = Arc::new(VkRenderPass::new(
            vk_device.clone(),
            swapchain.details.clone().choose_swapchain_format().format,
        )?);
        let graphics_pipeline = VkPipeline::egui_pipeline(
            vk_device.clone(),
            &[
                ShaderInformation::vertex_2d_information(
                    "shaders/2D_vertex_shader.spv".to_string(),
                ),
                ShaderInformation::fragment_2d_information(
                    "shaders/2D_fragment_shader.spv".to_string(),
                ),
            ],
            &[egui_descriptor_set_details.layout],
            &extent,
            render_pass.clone(),
        )?;
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
        let integration = EguiIntegration::new(window);
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
            compute_pipelines,
            compute_descriptor_set_details,
            compute_descriptor_allocator,
            egui_descriptor_allocator,
            egui_descriptor_set_details,
            graphics_pipeline,
            render_pass,
            allocated_image,
            framebuffers,
            image_details,
            egui_sampler,
            memory_allocator,
            frame_data,
            frame_idx: 0,
            render_area,
            integration,
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

            let swapchain_image_index = ImageIndex::new(
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

            let mesh_buffers = self
                .integration
                .run(
                    |ctx| {
                        egui::CentralPanel::default().show(&ctx, |ui| {
                            ui.label("Hello world!");
                            if ui.button("Click me").clicked() {
                                debug!("CLICKED");
                            }
                            if ui.button("WHAT THE HEEEEEEELLL").clicked() {
                                debug!("WHAT THE HEEEEELL");
                            }
                        });
                    },
                    window,
                )
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
                .collect::<Vec<_>>();

            self.record_command_buffer(
                frame_data,
                &swapchain_image_index,
                mesh_buffers.get(0).unwrap(),
            );

            let stage_masks = vec![PipelineStageFlags::VERTEX_SHADER];
            self.submit_queue(**self.graphics_queue, frame_data, &stage_masks);
            let image_indices = vec![swapchain_image_index.index];
            self.present_queue(
                **self.graphics_queue,
                &frame_data.render_semaphore,
                &image_indices,
            );
        }
    }

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
        mesh_buffers: &MeshBuffers,
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
            self.device
                .cmd_set_scissor(frame_data.command_buffer, 0, &self.scissors);
            let vertex_buffer = vec![mesh_buffers.vertex_buffer.buffer];
            self.device
                .cmd_bind_vertex_buffers(frame_data.command_buffer, 0, &vertex_buffer, &[0]);
            self.device.cmd_bind_index_buffer(
                frame_data.command_buffer,
                mesh_buffers.indices_buffer.buffer,
                0,
                IndexType::UINT32,
            );

            let extent = self.extent;
            let width = extent.width as f32;
            let height = extent.height as f32;

            // Create the orthographic projection matrix
            // Maps x from [0, width]   to [-1, 1]
            // Maps y from [0, height]  to [-1, 1] (adjust if Vulkan Y needs flipping relative to egui)
            // Common approach (assuming viewport handles Y inversion if needed):
            let sx = 2.0 / width;
            let sy = 2.0 / height; // Use -2.0 / height if you need to flip Y here
            let tx = -1.0;
            let ty = -1.0; // Use 1.0 if sy is negative (flipping Y)

            let clip_matrix = Matrix4::new(
                sx, 0.0, 0.0, 0.0, // Column 1
                0.0, sy, 0.0, 0.0, // Column 2
                0.0, 0.0, 1.0, 0.0, // Column 3 (maps Z=0 to Z=0, adjust Z scale/offset if needed)
                tx, ty, 0.0, 1.0, // Column 4
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

            image_util::image_transition(
                self.device.clone(),
                frame_data.command_buffer,
                self.graphics_queue.queue_family_index,
                self.image_details[image_index.index as usize].image,
                ImageLayout::UNDEFINED,
                ImageLayout::GENERAL,
            );

            self.device.cmd_bind_descriptor_sets(
                frame_data.command_buffer,
                PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline[0].pipeline_layout,
                0,
                &[*self.egui_descriptor_set_details],
                &[],
            );

            image_util::image_transition(
                self.device.clone(),
                frame_data.command_buffer,
                self.graphics_queue.queue_family_index,
                self.image_details[image_index.index as usize].image,
                ImageLayout::GENERAL,
                ImageLayout::PRESENT_SRC_KHR,
            );

            self.device.cmd_draw_indexed(
                frame_data.command_buffer,
                mesh_buffers.indices.len() as u32,
                1,
                0,
                0,
                0,
            );
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
