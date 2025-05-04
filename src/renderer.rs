use std::{fmt::Debug, iter::Sum, ops::Add, sync::Arc};

use anyhow::{Error, Result};
use ash::{
    ext::debug_utils,
    vk::{
        AttachmentLoadOp, BufferDeviceAddressCreateInfoEXT, BufferDeviceAddressInfo, ClearValue,
        ColorComponentFlags, CommandBuffer, CommandBufferBeginInfo, CommandBufferResetFlags,
        CommandBufferUsageFlags, CullModeFlags, DebugUtilsMessengerEXT, DynamicState, Extent2D,
        Fence, FrontFace, ImageLayout, IndexType, Offset2D, PipelineBindPoint, PipelineLayout,
        PipelineStageFlags, PolygonMode, PresentInfoKHR, PrimitiveTopology, Queue, Rect2D,
        RenderPassBeginInfo, SampleCountFlags, Semaphore, ShaderStageFlags, SubmitInfo,
        SubpassContents, Viewport,
    },
};
use log::debug;
use nalgebra::{Matrix4, Vector3, Vector4};
use vk_mem::{AllocatorCreateFlags, AllocatorCreateInfo};
use winit::window::Window;

const MAX_FRAMES: usize = 2;

use crate::{
    components::{
        allocation_types::{AllocatedImage, VkFrameBuffer},
        command_buffers::VkCommandPool,
        device::{self, VkDevice},
        frame_data::FrameData,
        instance::{self, VkInstance},
        memory_allocator::MemoryAllocator,
        pipeline::{
            create_color_blending_attachment_state, create_multisampling_state,
            create_rasterizer_state, ShaderInformation, VkPipeline,
        },
        queue::{QueueType, VkQueue},
        render_pass::VkRenderPass,
        surface,
        swapchain::{ImageDetails, KHRSwapchain},
    },
    egui::EguiRenderer,
    geom::{
        mesh::{Mesh, MeshBuffers},
        push_constants::PushConstant,
        triangle_push_constant,
        vertex_3d::Vertex3D,
        VertexAttributes,
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
    allocated_image: AllocatedImage,
    image_details: Vec<ImageDetails>,
    mesh_pipeline: VkPipeline,
    mesh_triangle_buffers: Vec<MeshBuffers<Vertex3D, u32>>,
    viewports: Vec<Viewport>,
    scissors: Vec<Rect2D>,
    framebuffers: Vec<VkFrameBuffer>,
    frame_data: Vec<FrameData>,
    frame_idx: usize,
    render_area: Rect2D,
    extent: Extent2D,
    command_pool: VkCommandPool,
    pub egui_renderer: EguiRenderer,
}

#[allow(unused)]
pub struct ImageIndex {
    pub index: u32,
    pub recreate_swapchain: bool,
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
        let mut alloc_info =
            AllocatorCreateInfo::new(&vk_instance, &vk_device, vk_device.physical_device);
        alloc_info.flags = AllocatorCreateFlags::BUFFER_DEVICE_ADDRESS;
        let memory_allocator = Arc::new(MemoryAllocator::new(vk_device.clone(), alloc_info));
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
            ImageLayout::UNDEFINED,
            ImageLayout::GENERAL,
            AttachmentLoadOp::CLEAR,
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

        let render_area = Rect2D::default()
            .offset(Offset2D::default().y(0).x(0))
            .extent(extent);
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

        let mesh_pipeline = VkPipeline::create_new_pipeline(
            vk_device.clone(),
            &[DynamicState::SCISSOR, DynamicState::VIEWPORT],
            PrimitiveTopology::TRIANGLE_LIST,
            ShaderStageFlags::VERTEX,
            &[
                ShaderInformation::new(
                    "/Users/zapzap/Projects/piplup/shaders/3_pos_vertex.spv".to_owned(),
                    ShaderStageFlags::VERTEX,
                    "main".to_string(),
                ),
                ShaderInformation::new(
                    "/Users/zapzap/Projects/piplup/shaders/triangle_fragment.spv".to_owned(),
                    ShaderStageFlags::FRAGMENT,
                    "main".to_string(),
                ),
            ],
            None,
            &extent,
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
        )?;

        let mesh = Mesh::<Vertex3D, u32> {
            vertices: vec![
                Vertex3D {
                    pos: Vector3::new(0.5, -0.5, 0.0),
                    color: Vector4::new(0.0, 0.0, 0.0, 1.0),
                    ..Default::default()
                },
                Vertex3D {
                    pos: Vector3::new(0.5, 0.5, 0.0),
                    color: Vector4::new(0.5, 0.5, 0.5, 1.0),
                    ..Default::default()
                },
                Vertex3D {
                    pos: Vector3::new(-0.5, -0.5, 0.0),
                    color: Vector4::new(1.0, 0.0, 0.0, 1.0),
                    ..Default::default()
                },
                Vertex3D {
                    pos: Vector3::new(-0.5, 0.5, 0.0),
                    color: Vector4::new(0.0, 1.0, 1.0, 0.0),
                    ..Default::default()
                },
            ],
            indices: vec![0, 1, 2, 2, 1, 3],
            texture_id: None,
            scissors: render_area,
            viewport: viewports[0],
        };

        let mesh_triangle_buffers = vec![MeshBuffers::new(
            mesh,
            |elements, usage, mem_usage, mem_flags| {
                memory_allocator
                    .create_buffer(
                        &elements,
                        &[graphics_queue.clone()],
                        usage,
                        mem_usage,
                        mem_flags,
                        &command_pool,
                    )
                    .unwrap()
            },
            |elements, usage, mem_usage, mem_flags| {
                memory_allocator
                    .create_buffer(
                        &elements,
                        &[graphics_queue.clone()],
                        usage,
                        mem_usage,
                        mem_flags,
                        &command_pool,
                    )
                    .unwrap()
            },
        )?];

        let egui_renderer = EguiRenderer::new(
            vk_device.clone(),
            window,
            memory_allocator.clone(),
            graphics_queue.clone(),
            extent,
            swapchain.details.clone().choose_swapchain_format().format,
        )?;
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
            mesh_pipeline,
            render_pass,
            allocated_image,
            framebuffers,
            image_details,
            memory_allocator,
            mesh_triangle_buffers,
            frame_data,
            frame_idx: 0,
            render_area,
            command_pool,
            viewports,
            scissors,
            extent,
            egui_renderer,
        })
    }

    pub fn display(&mut self, window: &Window) -> Result<()> {
        let frame_data = self.frame_data[self.frame_idx].clone();
        self.draw(&frame_data, window)?;
        self.frame_idx = self.frame_idx.add(1_usize) % MAX_FRAMES;
        Ok(())
    }

    fn draw(&mut self, frame_data: &FrameData, window: &Window) -> Result<()> {
        unsafe {
            self.device
                .wait_for_fences(&frame_data.render_fence, true, u64::MAX)?;
            self.device.reset_fences(&frame_data.render_fence)?;

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

            self.device.reset_command_buffer(
                frame_data.command_buffer,
                CommandBufferResetFlags::empty(),
            )?;

            let stage_masks = vec![
                PipelineStageFlags::VERTEX_SHADER,
                PipelineStageFlags::FRAGMENT_SHADER,
            ];

            self.record_command_buffer(frame_data, &image_index, window)
                .unwrap();
            self.egui_renderer.draw(
                frame_data.egui_command_buffer,
                &image_index,
                window,
                self.viewports.clone(),
                self.render_area,
                &self.framebuffers,
            )?;
            self.submit_queue(
                **self.graphics_queue,
                frame_data,
                &[frame_data.command_buffer, frame_data.egui_command_buffer],
                &stage_masks,
            );
            let image_indices = vec![image_index.index];
            self.present_queue(
                **self.graphics_queue,
                &frame_data.render_semaphore,
                &image_indices,
            );
        }
        Ok(())
    }

    fn record_command_buffer(
        &mut self,
        frame_data: &FrameData,
        image_index: &ImageIndex,
        window: &Window,
    ) -> Result<()> {
        unsafe {
            self.device.begin_command_buffer(
                frame_data.command_buffer,
                &CommandBufferBeginInfo::default().flags(CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;

            let clear_value = vec![ClearValue {
                color: ash::vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                },
            }];
            self.device.cmd_begin_render_pass(
                frame_data.command_buffer,
                &RenderPassBeginInfo::default()
                    .render_pass(**self.render_pass)
                    .framebuffer(*self.framebuffers[image_index.index as usize])
                    .render_area(self.render_area)
                    .clear_values(&clear_value),
                SubpassContents::INLINE,
            );
            for buffers in self.mesh_triangle_buffers.iter() {
                self.draw_geom(frame_data.command_buffer, buffers, window);
            }
            self.device.cmd_end_render_pass(frame_data.command_buffer);
            self.device.end_command_buffer(frame_data.command_buffer)?;
        }

        Ok(())
    }

    fn draw_geom<T: VertexAttributes + Debug, U: Sum + Debug>(
        &self,
        cmd: CommandBuffer,
        buffer: &MeshBuffers<T, U>,
        window: &Window,
    ) {
        unsafe {
            self.device
                .cmd_bind_pipeline(cmd, PipelineBindPoint::GRAPHICS, *self.mesh_pipeline);

            self.device.cmd_set_scissor(cmd, 0, &[self.render_area]);
            self.device.cmd_set_viewport(cmd, 0, &self.viewports);

            let push_constants_data = triangle_push_constant(buffer.vertex_buffer.address, window);
            self.device.cmd_push_constants(
                cmd,
                self.mesh_pipeline.pipeline_layout,
                ShaderStageFlags::VERTEX,
                0,
                &push_constants_data,
            );

            /*   self.device
            .cmd_bind_vertex_buffers(cmd, 0, &[buffer.vertex_buffer.buffer], &[0]); */
            self.device.cmd_bind_index_buffer(
                cmd,
                buffer.index_buffer.buffer,
                0,
                IndexType::UINT32,
            );

            self.device
                .cmd_draw_indexed(cmd, buffer.mesh.indices.len() as u32, 1, 0, 0, 0);
        };
    }

    #[allow(dead_code)]
    fn immediate_submit<F: FnOnce(&Renderer, CommandBuffer)>(&self, function: F) {
        let command = self.command_pool.single_time_command().unwrap();
        function(self, command);
        self.command_pool
            .end_single_time_command(self.graphics_queue.clone(), command);
    }

    fn submit_queue(
        &self,
        queue: Queue,
        frame_data: &FrameData,
        submit_cmd_buffers: &[CommandBuffer],
        stage_masks: &[PipelineStageFlags],
    ) {
        let submit_info = vec![SubmitInfo::default()
            .command_buffers(&submit_cmd_buffers)
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
