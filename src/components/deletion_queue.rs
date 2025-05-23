use std::{collections::VecDeque, marker::PhantomData, sync::Arc};

use ash::vk::Image;
use vk_mem::Allocation;

use super::{
    device::VkDevice,
    memory_allocator::{self, MemoryAllocator},
};

pub struct DestroyImageTask {
    pub image: Image,
    pub allocation: Allocation,
}

trait CleanUpTask<'a> {
    fn execute(&mut self, device: Arc<VkDevice>, malloc: Arc<MemoryAllocator>);
}

impl CleanUpTask<'static> for DestroyImageTask {
    fn execute(&mut self, _device: Arc<VkDevice>, malloc: Arc<MemoryAllocator>) {
        unsafe { malloc.destroy_image(self.image, &mut self.allocation) };
    }
}

pub enum FType {
    DEVICE(Box<dyn FnOnce(Arc<VkDevice>)>),
    MALLOC(Box<dyn FnOnce(Arc<MemoryAllocator>) + 'static>),
    TASK(Box<dyn CleanUpTask<'static>>),
}

pub struct DeletionQueue {
    device: Arc<VkDevice>,
    memory_allocator: Arc<MemoryAllocator>,
    queue: VecDeque<FType>,
}

impl DeletionQueue {
    pub fn new(device: Arc<VkDevice>, memory_allocator: Arc<MemoryAllocator>) -> Self {
        Self {
            device,
            memory_allocator,
            queue: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, func: FType) {
        self.queue.push_back(func);
    }

    pub fn flush(&mut self) {
        for i in self.queue.drain(..) {
            match i {
                FType::DEVICE(fn_once) => fn_once(self.device.clone()),
                FType::MALLOC(fn_once) => fn_once(self.memory_allocator.clone()),
                FType::TASK(mut fn_once) => {
                    fn_once.execute(self.device.clone(), self.memory_allocator.clone())
                }
            }
        }
        self.queue.clear();
    }
}
