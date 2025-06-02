use std::{
    collections::HashMap,
    fmt::Debug,
    ops::{Add, Deref},
    sync::Arc,
};

use anyhow::{Error, Result};
use ash::{
    ext::debug_utils,
    vk::{
        self, AttachmentLoadOp, BufferUsageFlags, ClearDepthStencilValue, ClearValue,
        ColorComponentFlags, CommandBuffer, CommandBufferBeginInfo, CommandBufferResetFlags,
        CommandBufferUsageFlags, CullModeFlags, DebugUtilsMessengerEXT,
        DescriptorSetLayoutCreateFlags, DescriptorType, DynamicState, Extent2D, Extent3D, Fence,
        Filter, Format, FrontFace, ImageAspectFlags, ImageLayout, ImageUsageFlags, IndexType,
        MemoryPropertyFlags, Offset2D, PipelineBindPoint, PipelineLayout, PipelineStageFlags,
        PolygonMode, PresentInfoKHR, PrimitiveTopology, Queue, Rect2D, RenderPassBeginInfo,
        SampleCountFlags, Sampler, Semaphore, ShaderStageFlags, SubmitInfo, SubpassContents,
        Viewport,
    },
    Device,
};
use egui::{frame, ImageData};
use log::debug;
use nalgebra::{Matrix4, Vector3, Vector4};
use vk_mem::{Alloc, AllocatorCreateFlags, AllocatorCreateInfo, MemoryUsage};
use winit::window::Window;

const MAX_FRAMES: usize = 2;

pub trait PackUnorm {
    fn pack_unorm4x8(&self) -> u32;
}

impl PackUnorm for Vector4<f32> {
    fn pack_unorm4x8(&self) -> u32 {
        let x = (self.x.clamp(0.0, 1.0) * 255.0).round() as u32;
        let y = (self.y.clamp(0.0, 1.0) * 255.0).round() as u32;
        let z = (self.z.clamp(0.0, 1.0) * 255.0).round() as u32;
        let w = (self.w.clamp(0.0, 1.0) * 255.0).round() as u32;

        (w << 24) | (z << 16) | (y << 8) | x
    }
}

use crate::{
    components::{
        allocation_types::{AllocatedImage, VkBuffer, VkFrameBuffer, IDENTIFIER},
        command_buffers::{self, VkCommandPool},
        deletion_queue::{self, DeletionQueue, DestroyBufferTask, DestroyImageTask, FType},
        descriptors::{
            DescriptorAllocator, DescriptorLayoutBuilder, DescriptorSetDetails, DescriptorWriter,
            PoolSizeRatio,
        },
        device::{self, VkDevice},
        frame_data::{self, FrameData, FrameResources},
        image_util::{copy_image_to_image, image_transition},
        instance::{self, VkInstance},
        memory_allocator::{AllocationUnit, MemoryAllocator},
        pipeline::{
            create_color_blending_attachment_state, create_multisampling_state,
            create_rasterizer_state, ShaderInformation, VkPipeline,
        },
        queue::{QueueType, VkQueue},
        render_pass::VkRenderPass,
        sampler::VkSampler,
        surface,
        swapchain::{ImageDetails, KHRSwapchain},
    },
    egui::EguiRenderer,
    geom::{
        assets::{self, MeshAsset},
        push_constants::PushConstant,
        scene::SceneData,
        triangle_push_constant,
        vertex_3d::Vertex3D,
        VertexAttributes,
    },
};

#[allow(unused)]
pub struct Renderer {
    pub instance: Arc<VkInstance>,
    debug_instance: debug_utils::Instance,
    debugger: DebugUtilsMessengerEXT,
    device: Arc<VkDevice>,
    graphics_queue: Arc<VkQueue>,
    presentation_queue: Arc<VkQueue>,
    swapchain: Arc<KHRSwapchain>,
    render_pass: Arc<VkRenderPass>,
    memory_allocator: Arc<MemoryAllocator>,
    draw_image: AllocatedImage,
    depth_image: AllocatedImage,
    descriptor_allocator: DescriptorAllocator,
    descriptor_layout_builder: DescriptorLayoutBuilder<'static>,
    descriptor_writer: DescriptorWriter,
    single_image_descriptor: DescriptorSetDetails,
    gltf_pipeline: VkPipeline,
    gltf_buffers: Vec<MeshAsset<Vertex3D>>,
    viewports: Vec<Viewport>,
    scissors: Vec<Rect2D>,
    swapchain_image_details: Vec<ImageDetails>,
    framebuffers: HashMap<IDENTIFIER, Vec<VkFrameBuffer>>,
    frame_data: Vec<FrameData>,
    frame_idx: usize,
    scene_data: SceneData,
    render_area: Rect2D,
    extent: Extent2D,
    command_pool: VkCommandPool,
    main_deletion_queue: DeletionQueue,
    pub checkboard_image: AllocatedImage,
    pub egui_renderer: EguiRenderer,
}

#[allow(unused)]
pub struct ImageIndex {
    pub index: u32,
    pub recreate_swapchain: bool,
}

impl Deref for ImageIndex {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.index
    }
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
        let command_pool = VkCommandPool::new(graphics_queue.clone());
        let extent = swapchain.details.clone().choose_swapchain_extent(window);
        let mut alloc_info =
            AllocatorCreateInfo::new(&vk_instance, &vk_device, vk_device.physical_device);
        alloc_info.flags = AllocatorCreateFlags::BUFFER_DEVICE_ADDRESS;
        let memory_allocator = Arc::new(MemoryAllocator::new(
            vk_device.clone(),
            &[graphics_queue.clone()],
            alloc_info,
        ));
        #[allow(unused_mut)]
        let mut main_deletion_queue =
            DeletionQueue::new(vk_device.clone(), memory_allocator.clone());
        let draw_image = memory_allocator.create_image(
            Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            },
            Format::R16G16B16A16_SFLOAT,
            None,
            ImageUsageFlags::STORAGE | ImageUsageFlags::COLOR_ATTACHMENT,
            ImageAspectFlags::COLOR,
            false,
        )?;
        let allocation = draw_image.allocation;
        let draw_image = draw_image.unit.get_cloned::<AllocatedImage>();
        main_deletion_queue.enqueue(FType::TASK(Box::new(DestroyImageTask {
            image: draw_image.image_details.image,
            allocation,
        })));
        main_deletion_queue.enqueue(FType::DEVICE(Box::new(move |device| unsafe {
            device.destroy_image_view(draw_image.image_details.image_view, None)
        })));
        let mut framebuffers: HashMap<IDENTIFIER, Vec<VkFrameBuffer>> = HashMap::new();
        let depth_image = memory_allocator.create_image(
            Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            },
            Format::D32_SFLOAT,
            None,
            ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            ImageAspectFlags::DEPTH,
            false,
        )?;
        let depth_allocation = depth_image.allocation;
        let depth_image = depth_image.unit.get_copied::<AllocatedImage>();
        main_deletion_queue.enqueue(FType::TASK(Box::new(DestroyImageTask {
            image: depth_image.image_details.image,
            allocation: depth_allocation,
        })));
        main_deletion_queue.enqueue(FType::DEVICE(Box::new(move |device| unsafe {
            device.destroy_image_view(depth_image.image_details.image_view, None)
        })));

        let white = Vector4::<f32>::new(1.0, 1.0, 1.0, 1.0).pack_unorm4x8();
        let white_image = memory_allocator
            .create_image_with_data(
                &[white],
                Extent3D {
                    width: 1,
                    height: 1,
                    depth: 1,
                },
                Format::R8G8B8A8_UNORM,
                ImageUsageFlags::SAMPLED,
                ImageAspectFlags::COLOR,
                &command_pool,
                false,
            )
            .unwrap();

        let grey = Vector4::<f32>::new(0.66, 0.66, 0.66, 1.0).pack_unorm4x8();
        let grey_image = memory_allocator
            .create_image_with_data(
                &[grey],
                Extent3D {
                    width: 1,
                    height: 1,
                    depth: 1,
                },
                Format::R8G8B8A8_UNORM,
                ImageUsageFlags::SAMPLED,
                ImageAspectFlags::COLOR,
                &command_pool,
                false,
            )
            .unwrap();

        let black = Vector4::new(0.0, 0.0, 0.0, 0.0).pack_unorm4x8();
        let black_image = memory_allocator
            .create_image_with_data(
                &[black],
                Extent3D {
                    width: 1,
                    height: 1,
                    depth: 1,
                },
                Format::R8G8B8A8_UNORM,
                ImageUsageFlags::SAMPLED,
                ImageAspectFlags::COLOR,
                &command_pool,
                false,
            )
            .unwrap();

        let magenta = Vector4::new(1.0, 0.0, 1.0, 1.0).pack_unorm4x8();
        let magenta_image = memory_allocator
            .create_image_with_data(
                &[magenta],
                Extent3D {
                    width: 1,
                    height: 1,
                    depth: 1,
                },
                Format::R8G8B8A8_UNORM,
                ImageUsageFlags::SAMPLED,
                ImageAspectFlags::COLOR,
                &command_pool,
                false,
            )
            .unwrap();

        let mut pixels = [0 as u32; 16 * 16];
        for i in 0..16 {
            for j in 0..16 {
                pixels[j * 16 + i] = if ((i % 2) ^ (j % 2)).eq(&1) {
                    magenta
                } else {
                    black
                }
            }
        }
        debug!("PIXELS={pixels:?}");
        let error_checkboard = memory_allocator.create_image_with_data(
            &pixels,
            Extent3D {
                width: 16,
                height: 16,
                depth: 1,
            },
            Format::R8G8B8A8_UNORM,
            ImageUsageFlags::SAMPLED,
            ImageAspectFlags::COLOR,
            &command_pool,
            false,
        )?;

        let default_nearest_sampler =
            VkSampler::with_filter(vk_device.clone(), Filter::NEAREST, Filter::NEAREST);

        let default_linear_sampler =
            VkSampler::with_filter(vk_device.clone(), Filter::LINEAR, Filter::LINEAR);

        let render_pass = Arc::new(VkRenderPass::new(
            vk_device.clone(),
            swapchain.details.clone().choose_swapchain_format().format,
            ImageLayout::UNDEFINED,
            ImageLayout::TRANSFER_SRC_OPTIMAL,
            AttachmentLoadOp::CLEAR,
            true,
        )?);
        let draw_framebuffers = VkFrameBuffer::create_framebuffer(
            IDENTIFIER::DRAW,
            vk_device.clone(),
            render_pass.clone(),
            extent,
            &[draw_image.image_details, depth_image.image_details],
        );
        let mut descriptor_allocator = DescriptorAllocator::new(
            vk_device.clone(),
            16,
            vec![PoolSizeRatio::new(
                DescriptorType::COMBINED_IMAGE_SAMPLER,
                1.0,
            )],
        );
        let scene_data = SceneData::default();
        /* let scene_descriptor = descriptor_allocator.write_image_descriptors(
            &draw_image.image_details.image_view,
            ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT,
            DescriptorType::UNIFORM_BUFFER,
            None,
        )?;*/
        /* let depth_framebuffers = VkFrameBuffer::create_framebuffers(
            IDENTIFIER::DEPTH,
            vk_device.clone(),
            render_pass.clone(),
            extent,
            &[draw_image.image_details],
        ) */
        let swapchain_image_details = swapchain.create_image_details()?;
        framebuffers.insert(IDENTIFIER::DRAW, vec![draw_framebuffers]);
        let mut frame_data: Vec<FrameData> = Vec::new();
        for _i in 0..MAX_FRAMES {
            frame_data.push(FrameData::new(
                vk_device.clone(),
                memory_allocator.clone(),
                graphics_queue.clone(),
            ));
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
        let mut writer = DescriptorWriter::new();
        let mut descriptor_layout_builder = DescriptorLayoutBuilder::new();
        descriptor_layout_builder.add_binding(0, DescriptorType::COMBINED_IMAGE_SAMPLER);
        let single_image_layout = descriptor_layout_builder.build(
            vk_device.clone(),
            ShaderStageFlags::FRAGMENT,
            DescriptorSetLayoutCreateFlags::empty(),
        );
        descriptor_layout_builder.clear();
        let single_image_descriptor =
            descriptor_allocator.allocate(vk_device.clone(), &[single_image_layout]);
        writer.write_image(
            0,
            error_checkboard
                .unit
                .get_copied::<AllocatedImage>()
                .image_details
                .image_view,
            Some(default_nearest_sampler),
            ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            DescriptorType::COMBINED_IMAGE_SAMPLER,
        );
        writer.update_set(vk_device.clone(), single_image_descriptor[0]);
        writer.clear();

        let gltf_pipeline = VkPipeline::create_new_pipeline(
            vk_device.clone(),
            &[DynamicState::SCISSOR, DynamicState::VIEWPORT],
            PrimitiveTopology::TRIANGLE_LIST,
            ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT,
            &[
                ShaderInformation::new(
                    "/Users/zapzap/Projects/piplup/shaders/3_pos_vertex.spv".to_owned(),
                    ShaderStageFlags::VERTEX,
                    "main".to_string(),
                ),
                ShaderInformation::new(
                    "/Users/zapzap/Projects/piplup/shaders/tex_image.spv".to_owned(),
                    ShaderStageFlags::FRAGMENT,
                    "main".to_string(),
                ),
            ],
            Some(&single_image_descriptor.layout),
            &extent,
            Some(PushConstant::<Matrix4<f32>>::default()),
            [].to_vec(),
            [].to_vec(),
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
        let gltf_buffers = assets::MeshAsset::<Vertex3D>::load_gltf_meshes(
            "/Users/zapzap/Projects/piplup/assets/basicmesh.glb",
            scissors[0],
            viewports[0],
            memory_allocator.clone(),
            &[graphics_queue.clone()],
            command_pool.clone(),
        )?;
        debug!("1");
        /*  let mesh_triangle_buffers = vec![MeshBuffers::new(
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
        )?]; */

        let egui_renderer = EguiRenderer::new(
            vk_device.clone(),
            window,
            memory_allocator.clone(),
            graphics_queue.clone(),
            extent,
            swapchain.details.clone().choose_swapchain_format().format,
            swapchain_image_details.clone(),
        )?;

        Ok(Self {
            instance: vk_instance,
            debug_instance,
            debugger,
            device: vk_device,
            graphics_queue,
            presentation_queue,
            swapchain,
            main_deletion_queue,
            //    compute_pipelines,
            //   compute_descriptor_set_details,
            //  compute_descriptor_allocator,
            gltf_pipeline,
            render_pass,
            draw_image,
            depth_image,
            descriptor_allocator,
            descriptor_layout_builder,
            descriptor_writer: writer,
            single_image_descriptor,
            swapchain_image_details,
            framebuffers,
            memory_allocator,
            gltf_buffers,
            scene_data,
            frame_data,
            frame_idx: 0,
            render_area,
            command_pool,
            viewports,
            scissors,
            extent,
            checkboard_image: error_checkboard.unit.get_copied::<AllocatedImage>(),
            egui_renderer,
        })
    }

    pub fn display(&mut self, window: &Window) -> Result<()> {
        self.draw(self.frame_idx, window)?;
        self.frame_idx = self.frame_idx.add(1_usize) % MAX_FRAMES;
        Ok(())
    }

    fn draw(&mut self, frame_idx: usize, window: &Window) -> Result<()> {
        unsafe {
            self.device.wait_for_fences(
                &self.frame_data[frame_idx].render_fence,
                true,
                u64::MAX,
            )?;
            self.device
                .reset_fences(&self.frame_data[frame_idx].render_fence)?;

            let image_index = ImageIndex::new(
                self.swapchain
                    .s_device
                    .acquire_next_image(
                        **self.swapchain,
                        u64::MAX,
                        self.frame_data[frame_idx].swapchain_semaphore[0],
                        Fence::null(),
                    )
                    .unwrap(),
            );

            self.device.reset_command_buffer(
                self.frame_data[frame_idx].command_buffer,
                CommandBufferResetFlags::empty(),
            )?;

            let stage_masks = vec![
                PipelineStageFlags::VERTEX_SHADER,
                PipelineStageFlags::FRAGMENT_SHADER,
            ];

            {
                Self::record_command_buffer(
                    self.frame_data[frame_idx].command_buffer,
                    &image_index,
                    window,
                    &mut self.frame_data[frame_idx].frame_resources,
                    &self.device.clone(),
                    &self.swapchain_image_details,
                    &self.draw_image,
                    &self.graphics_queue.clone(),
                    &self.render_area,
                    &self.viewports,
                    &self.single_image_descriptor,
                    &self.gltf_pipeline,
                    &self.gltf_buffers,
                    &self.memory_allocator,
                    &self.extent,
                    &self.render_pass,
                    &self.depth_image,
                    &self.framebuffers,
                )
                .unwrap();
            }
            self.egui_renderer.draw(
                self.frame_data[frame_idx].egui_command_buffer,
                &image_index,
                window,
                self.viewports.clone(),
                self.render_area,
            )?;
            let frame_data = &self.frame_data[frame_idx];
            self.submit_queue(
                **self.graphics_queue,
                frame_idx,
                &[
                    self.frame_data[frame_idx].command_buffer,
                    self.frame_data[frame_idx].egui_command_buffer,
                ],
                &stage_masks,
            );
            let image_indices = vec![image_index.index];
            self.present_queue(
                **self.graphics_queue,
                &self.frame_data[frame_idx].render_semaphore,
                &image_indices,
            );
            debug!("self.frame_idx={:?}", self.frame_idx);
            self.frame_data[self.frame_idx]
                .frame_resources
                .deletion_queue
                .flush();
            self.frame_data[self.frame_idx]
                .frame_resources
                .descriptor_allocator
                .borrow_mut()
                .clear_pools();
        }
        Ok(())
    }

    fn record_command_buffer(
        cmd: CommandBuffer,
        image_index: &ImageIndex,
        window: &Window,
        frame_resources: &mut FrameResources,
        device: &Arc<VkDevice>,
        swapchain_image_details: &[ImageDetails],
        draw_image: &AllocatedImage,
        graphics_queue: &Arc<VkQueue>,
        render_area: &Rect2D,
        viewports: &[Viewport],
        descriptor_set: &DescriptorSetDetails,
        gltf_pipeline: &VkPipeline,
        gltf_buffers: &[MeshAsset<Vertex3D>],
        memory_allocator: &Arc<MemoryAllocator>,
        extent: &Extent2D,
        render_pass: &Arc<VkRenderPass>,
        depth_image: &AllocatedImage,
        framebuffers: &HashMap<IDENTIFIER, Vec<VkFrameBuffer>>,
    ) -> Result<()> {
        unsafe {
            let current_image = swapchain_image_details[**image_index as usize];
            device.begin_command_buffer(
                cmd,
                &CommandBufferBeginInfo::default().flags(CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;

            let clear_value = vec![
                ClearValue {
                    color: ash::vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 0.0],
                    },
                },
                ClearValue {
                    depth_stencil: ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];
            device.cmd_begin_render_pass(
                cmd,
                &RenderPassBeginInfo::default()
                    .render_pass(***render_pass) // Dereference VkRenderPass
                    .framebuffer(*framebuffers.get(&IDENTIFIER::DRAW).unwrap()[0])
                    .render_area(*render_area)
                    .clear_values(&clear_value),
                SubpassContents::INLINE,
            );
            Self::draw_geom::<Vertex3D>(
                cmd,
                window,
                frame_resources,
                gltf_buffers,
                memory_allocator,
                descriptor_set,
                device,
                extent,
                viewports,
                gltf_pipeline,
                render_area,
                draw_image,
                graphics_queue,
            )?;
            device.cmd_end_render_pass(cmd);
            image_transition(
                device.clone(),
                cmd,
                graphics_queue.queue_family_index,
                current_image.image,
                ImageLayout::UNDEFINED,
                ImageLayout::TRANSFER_DST_OPTIMAL,
            );
            let extent = Extent2D::default()
                .width(draw_image.extent.width)
                .height(draw_image.extent.height);
            copy_image_to_image(
                &device,
                cmd,
                draw_image.image_details.image,
                current_image.image,
                extent,
                extent, // Pass extent directly
            );
            image_transition(
                device.clone(),
                cmd,
                graphics_queue.queue_family_index,
                current_image.image,
                ImageLayout::TRANSFER_DST_OPTIMAL,
                ImageLayout::GENERAL,
            );

            device.end_command_buffer(cmd)?;
        }

        Ok(())
    }

    fn draw_geom<T: VertexAttributes + Debug>(
        cmd: CommandBuffer,
        window: &Window,
        frame_resources: &mut FrameResources,
        gltf_buffers: &[MeshAsset<Vertex3D>],
        memory_allocator: &Arc<MemoryAllocator>,
        descriptor_set: &DescriptorSetDetails,
        device: &Arc<VkDevice>,
        extent: &Extent2D,
        viewports: &[Viewport],
        gltf_pipeline: &VkPipeline,
        render_area: &Rect2D,
        draw_image: &AllocatedImage,
        graphics_queue: &Arc<VkQueue>,
    ) -> Result<()> {
        unsafe {
            let scene_data_size = size_of::<SceneData>();
            let gpu_scene_data_buffer = memory_allocator.allocate_single_buffer(
                scene_data_size as u64,
                &[graphics_queue.clone()],
                BufferUsageFlags::UNIFORM_BUFFER | BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                vk_mem::MemoryUsage::CpuToGpu,
                MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::DEVICE_LOCAL,
            )?;
            let scene_data_allocation = gpu_scene_data_buffer.allocation;
            let gpu_scene_data_buffer = gpu_scene_data_buffer.unit.get_copied::<VkBuffer>();

            frame_resources
                .deletion_queue
                .enqueue(FType::TASK(Box::new(DestroyBufferTask {
                    buffer: *gpu_scene_data_buffer,
                    allocation: scene_data_allocation,
                })));

            //write buffer with self.scene data
            // HERE
            //
            // Descriptors
            let descriptor_write = frame_resources
                .descriptor_allocator
                .borrow_mut()
                .write_buffer_descriptors(
                    &gpu_scene_data_buffer,
                    scene_data_size as u64,
                    ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT,
                    DescriptorType::UNIFORM_BUFFER,
                )?;
            device.cmd_bind_pipeline(cmd, PipelineBindPoint::GRAPHICS, **gltf_pipeline);
            device.cmd_bind_descriptor_sets(
                cmd,
                PipelineBindPoint::GRAPHICS,
                gltf_pipeline.pipeline_layout,
                0,
                descriptor_set,
                &[],
            );

            device.cmd_set_scissor(cmd, 0, &[*render_area]);
            device.cmd_set_viewport(cmd, 0, viewports);

            let push_constants_data =
                triangle_push_constant(gltf_buffers[2].mesh_buffers.vertex_buffer.address, *extent);
            device.cmd_push_constants(
                cmd,
                gltf_pipeline.pipeline_layout,
                ShaderStageFlags::VERTEX | ShaderStageFlags::FRAGMENT,
                0,
                &push_constants_data,
            );

            device.cmd_bind_index_buffer(
                cmd,
                gltf_buffers[2].mesh_buffers.index_buffer.buffer,
                0,
                IndexType::UINT32,
            );

            device.cmd_draw_indexed(
                cmd,
                gltf_buffers[2].surfaces[0].count as u32,
                1,
                gltf_buffers[2].surfaces[0].start_index,
                0,
                0,
            );
        };
        Ok(())
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
        frame_idx: usize, // Added frame_idx
        submit_cmd_buffers: &[CommandBuffer],
        stage_masks: &[PipelineStageFlags],
    ) {
        let frame_data = &self.frame_data[frame_idx]; // Access frame_data using index
        let submit_info = vec![SubmitInfo::default()
            .command_buffers(submit_cmd_buffers)
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
