use std::{io::Error, ops::Add, sync::Arc};

use ash::{
    ext::debug_utils,
    vk::{
        ClearValue, CommandBuffer, CommandBufferBeginInfo, CommandBufferResetFlags, CommandBufferUsageFlags, DebugUtilsMessengerEXT, DescriptorType, Fence, Framebuffer, RenderPassBeginInfo, ShaderStageFlags, SubpassContents
    },
};
use log::debug;
use vk_mem::{Allocator, AllocatorCreateInfo};
use winit::window::{self, Window};

const MAX_FRAMES: usize = 2;

use crate::{
    components::{
        allocated_image::AllocatedImage, buffers::VkFrameBuffer, descriptors::{DescriptorAllocator, DescriptorSetDetails, PoolSizeRatio}, device::{self, VkDevice}, frame_data::FrameData, instance::{self, VkInstance}, pipeline::{ShaderInformation, VkPipeline}, queue::{QueueType, VkQueue}, render_pass::VkRenderPass, surface, swapchain::{ImageDetails, KHRSwapchain}
    },
    egui::integration::EguiIntegration,
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
    vk_mem_allocator: Arc<Allocator>,
    graphics_pipeline: Vec<VkPipeline>,
    descriptor_allocator: DescriptorAllocator,
    compute_descriptor_set_details: DescriptorSetDetails,
    allocated_image: AllocatedImage,
    image_details: Vec<ImageDetails>,
    framebuffers: Vec<VkFrameBuffer>,
    frame_data: Vec<FrameData>,
    frame_idx: usize,
    integration: EguiIntegration,
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

        let vk_mem_allocator = Arc::new(unsafe {
            vk_mem::Allocator::new(AllocatorCreateInfo::new(
                &vk_instance,
                &vk_device,
                vk_device.physical_device,
            ))
            .unwrap()
        });
        let image_details = swapchain.create_image_details()?;
        let allocated_image = swapchain.create_allocated_image(vk_mem_allocator.clone())?;
        let descriptor_allocator = DescriptorAllocator::new(
            vk_device.clone(),
            10,
            vec![PoolSizeRatio::new(DescriptorType::STORAGE_IMAGE, 1.0)],
        );
        let compute_descriptor_set_details = descriptor_allocator.get_compute_descriptors(
            &allocated_image,
            ShaderStageFlags::COMPUTE,
            DescriptorType::STORAGE_IMAGE,
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
        let graphics_pipeline = VkPipeline::graphics_pipelines(
            vk_device.clone(),
            &[ShaderInformation::vertex_2d_information(
                "shaders/2D_vertex_shader.spv".to_string(),
            )],
            &swapchain.details.clone().choose_swapchain_extent(&window),
            render_pass.clone(),
        )?;
        let framebuffers = VkFrameBuffer::create_framebuffers(
            vk_device.clone(),
            render_pass.clone(),
            swapchain.details.clone().choose_swapchain_extent(window),
            &image_details,
        );
        let mut frame_data: Vec<FrameData> = Vec::new();
        for _i in 0..MAX_FRAMES {
            frame_data.push(FrameData::new(
                vk_device.clone(),
                graphics_queue.queue_family_index,
            ));
        }
        let integration = EguiIntegration::new(window);

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
            descriptor_allocator,
            graphics_pipeline,
            render_pass,
            allocated_image,
            framebuffers,
            image_details,
            vk_mem_allocator,
            frame_data,
            frame_idx: 0,
            integration,
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
            let run = self.integration.run(
                |ctx| {
                    egui::CentralPanel::default().show(&ctx, |ui| {
                        ui.label("Hello world!");
                        if ui.button("Click me").clicked() {
                            debug!("CLICKED");
                        }
                    });
                },
                window,
            );
            self.device
                .reset_command_buffer(frame_data.command_buffer, CommandBufferResetFlags::empty())
                .unwrap();

            self.record_command_buffer(frame_data.command_buffer, swapchain_image_index);

            self.submit_queue();
            self.present_queue();
        }
    }

    fn record_command_buffer(&self, command_buffer: CommandBuffer, image_index: ImageIndex) {
        unsafe {
            let begin_info =
                CommandBufferBeginInfo::default().flags(CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device
                .begin_command_buffer(command_buffer, &begin_info)
                .unwrap();
            
            let clear_value = vec![ClearValue {
                color: ash::vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] }
            }];

            let render_pass_begin_info = RenderPassBeginInfo::default().render_pass(**self.render_pass)
                .framebuffer(*self.framebuffers[image_index.index as usize])
                .clear_values(&clear_value);
            
            self.device.cmd_begin_render_pass(command_buffer, &render_pass_begin_info, SubpassContents::INLINE);
                
            self.device.cmd_end_render_pass(command_buffer);
            //           self.device.cmd_begin_render_pass(ccommand_buffer, , contents);
            self.device.end_command_buffer(command_buffer).unwrap();
        }
    }

    fn submit_queue(&self) {}

    fn present_queue(&self) {}
}
