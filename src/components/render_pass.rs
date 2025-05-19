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
        initial_layout: ImageLayout,
        final_layout: ImageLayout,
        attachment_load_op: AttachmentLoadOp,
        depth: bool,
    ) -> Result<VkRenderPass, Error> {
        let attachment = create_attachment(
            format,
            initial_layout,
            final_layout,
            AttachmentStoreOp::STORE,
            attachment_load_op,
        );
        let attachment_ref = vec![create_attachment_ref(
            ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            0,
        )];
        let depth_attachment = if depth {
            Some(create_attachment(
                Format::D32_SFLOAT,
                ImageLayout::UNDEFINED,
                ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
                AttachmentStoreOp::DONT_CARE,
                AttachmentLoadOp::CLEAR,
            ))
        } else {
            None
        };
        let depth_ref = if depth {
            Some(create_attachment_ref(
                ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                1,
            ))
        } else {
            None
        };

        let subpass_description = create_subpass_description(&attachment_ref, depth_ref.as_ref());

        let subpass_dependency = create_subpass_dependency(
            DependencyFlags::BY_REGION,
            0,
            0,
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            PipelineStageFlags::FRAGMENT_SHADER,
            AccessFlags::COLOR_ATTACHMENT_READ,
            AccessFlags::SHADER_READ,
        );
        let attachments = if depth {
            vec![attachment, depth_attachment.unwrap()]
        } else {
            vec![attachment]
        };
        let descriptions = vec![subpass_description];
        Ok(unsafe {
            Self {
                render_pass: device
                    .create_render_pass(
                        &render_pass_create_info(
                            &attachments,
                            &descriptions,
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

fn create_attachment(
    image_format: Format,
    initial_layout: ImageLayout,
    final_layout: ImageLayout,
    store_op: AttachmentStoreOp,
    load_op: AttachmentLoadOp,
) -> AttachmentDescription {
    AttachmentDescription::default()
        .format(image_format)
        .samples(SampleCountFlags::TYPE_1)
        .load_op(load_op)
        .store_op(store_op)
        .stencil_load_op(AttachmentLoadOp::CLEAR)
        .stencil_store_op(AttachmentStoreOp::STORE)
        .initial_layout(initial_layout)
        .final_layout(final_layout)
}

fn create_attachment_ref(layout: ImageLayout, attachment_index: u32) -> AttachmentReference {
    AttachmentReference::default()
        .layout(layout)
        .attachment(attachment_index)
}

fn create_subpass_description<'a>(
    attachments: &'a [AttachmentReference],
    depth_attachment: Option<&'a AttachmentReference>,
) -> SubpassDescription<'a> {
    let mut subpass = SubpassDescription::default()
        .color_attachments(attachments)
        .pipeline_bind_point(PipelineBindPoint::GRAPHICS)
        .color_attachments(attachments);

    if let Some(att) = depth_attachment {
        subpass = subpass.depth_stencil_attachment(&att);
    }
    subpass
}
