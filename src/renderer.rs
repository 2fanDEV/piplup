use std::{io::Error, sync::Arc};

use ash::{
    ext::debug_utils,
    vk::{DebugUtilsMessengerEXT, DescriptorType, Queue, ShaderStageFlags},
};
use vk_mem::{Allocator, AllocatorCreateInfo};
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::{self, Window},
};

use crate::components::{
    descriptors::{DescriptorAllocator, DescriptorSetDetails, PoolSizeRatio},
    device::{self, VkDevice},
    instance::{self, VkInstance},
    pipeline::{ShaderInformation, VkPipeline},
    queue::{QueueType, VkQueue},
    render_pass::VkRenderPass,
    surface,
    swapchain::KHRSwapchain,
};

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
        let allocated_image = swapchain.create_allocated_image(vk_mem_allocator.clone())?;
        let descriptor_allocator = DescriptorAllocator::new(
            vk_device.clone(),
            10,
            vec![PoolSizeRatio::new(DescriptorType::STORAGE_IMAGE, 1.0)],
        );
        let compute_descriptor_set_details = descriptor_allocator
            .get_compute_descriptors(
                allocated_image,
                ShaderStageFlags::COMPUTE,
                DescriptorType::STORAGE_IMAGE,
            )
            .unwrap();
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
            vk_mem_allocator,
        })
    }

    pub fn draw(&self) {}
}
