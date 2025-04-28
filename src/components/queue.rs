use std::{io::Error, ops::Deref, sync::Arc};

use ash::vk::Queue;

use super::{device::{QueueFamilyIndices, VkDevice}, surface::KHRSurface};

#[allow(warnings)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueueType {
    GRAPHICS_QUEUE,
    PRESENT_QUEUE,
}

#[derive(Clone)]
pub struct VkQueue {
    pub queue: Queue,
    pub queue_family_index: u32,
    pub device: Arc<VkDevice>,
    pub queue_type: QueueType,
}

impl Deref for VkQueue {
    type Target = Queue;

    fn deref(&self) -> &Self::Target {
        &self.queue
    }
}

impl VkQueue {
    pub fn new(
        device: Arc<VkDevice>,
        surface: Arc<KHRSurface>,
        queue_type: QueueType
    ) -> Result<Self, Error> {
        let queue_family_indices = QueueFamilyIndices::find_queue_family_indices(
            device.physical_device,
            &device.instance,
            surface,
        );
        let queue_family_index = match queue_type {
            QueueType::GRAPHICS_QUEUE => queue_family_indices.graphics_q_idx.unwrap(),
            QueueType::PRESENT_QUEUE => queue_family_indices.presentation_q_idx.unwrap(),
        };
        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };
        Ok(Self {
            queue,
            queue_family_index,
            device,
            queue_type,
        })
    }
}
