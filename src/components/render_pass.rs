use std::{io::Error, ops::Deref, sync::Arc};

use ash::vk::{
    AccessFlags, AttachmentDescription, AttachmentLoadOp, AttachmentReference, AttachmentStoreOp,
    DependencyFlags, Format, ImageLayout, PipelineBindPoint, PipelineStageFlags, RenderPass,
    RenderPassCreateInfo, SampleCountFlags, SubpassDependency, SubpassDescription,
};

use super::device::VkDevice;

#[allow(unused)]
pub struct VkRenderPass {
    render_pass: RenderPass,
    device: Arc<VkDevice>,
    format: Format,
}

impl Deref for VkRenderPass {
    type Target = RenderPass;

    fn deref(&self) -> &Self::Target {
        &self.render_pass
    }
}

impl VkRenderPass {
    pub fn new(
        device: Arc<VkDevice>,
        format: Format,
        attachment_load_op: AttachmentLoadOp,
    ) -> Result<VkRenderPass, Error> {
        let color_attachment = create_attachment(format, attachment_load_op);
        let color_attachment_ref = vec![create_attachment_ref()];
        let subpass_description = create_subpass_description(&color_attachment_ref);
        let subpass_dependency = create_subpass_dependency(
            DependencyFlags::BY_REGION,
            0,
            0,
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            PipelineStageFlags::FRAGMENT_SHADER,
            AccessFlags::COLOR_ATTACHMENT_READ,
            AccessFlags::SHADER_READ,
        );
        Ok(unsafe {
            Self {
                render_pass: device
                    .create_render_pass(
                        &render_pass_create_info(
                            &[color_attachment],
                            &[subpass_description],
                            &[subpass_dependency],
                        ),
                        None,
                    )
                    .unwrap(),
                device,
                format,
            }
        })
    }
}

fn render_pass_create_info<'a>(
    attachments: &'a [AttachmentDescription],
    description: &'a [SubpassDescription],
    dependencies: &'a [SubpassDependency],
) -> RenderPassCreateInfo<'a> {
    RenderPassCreateInfo::default()
        .attachments(attachments)
        .subpasses(description)
        .dependencies(dependencies)
}

fn create_subpass_dependency(
    flag: DependencyFlags,
    src_subpass: u32,
    dst_subpass: u32,
    src_stage_mask: PipelineStageFlags,
    dst_stage_mask: PipelineStageFlags,
    src_access_mask: AccessFlags,
    dst_access_mask: AccessFlags,
) -> SubpassDependency {
    SubpassDependency::default()
        .dependency_flags(flag)
        .src_subpass(src_subpass)
        .dst_subpass(dst_subpass)
        .src_stage_mask(src_stage_mask)
        .src_access_mask(src_access_mask)
        .dst_access_mask(dst_access_mask)
        .dst_stage_mask(dst_stage_mask)
}

fn create_attachment(image_format: Format, load_op: AttachmentLoadOp) -> AttachmentDescription {
    AttachmentDescription::default()
        .format(image_format)
        .samples(SampleCountFlags::TYPE_1)
        .load_op(load_op)
        .store_op(AttachmentStoreOp::STORE)
        .stencil_load_op(AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(AttachmentStoreOp::DONT_CARE)
        .initial_layout(ImageLayout::UNDEFINED)
        .final_layout(ImageLayout::PRESENT_SRC_KHR)
}

fn create_attachment_ref() -> AttachmentReference {
    AttachmentReference::default().layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
}

fn create_subpass_description(color_attachments: &[AttachmentReference]) -> SubpassDescription<'_> {
    SubpassDescription::default()
        .color_attachments(color_attachments)
        .pipeline_bind_point(PipelineBindPoint::GRAPHICS)
        .color_attachments(color_attachments)
}
