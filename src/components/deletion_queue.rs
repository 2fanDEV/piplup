use std::{cell::RefCell, collections::VecDeque, marker::PhantomData, rc::Rc, sync::Arc};

use ash::vk::{CommandPool, DescriptorPool, Image};
use log::debug;
use vk_mem::Allocation;

use super::{
    descriptors::DescriptorAllocator, device::VkDevice, memory_allocator::MemoryAllocator,
};

pub struct DestroyImageTask {
    pub image: Image,
    pub allocation: Allocation,
}

pub struct DestroyDescriptorPools {
    pub allocator: RefCell<DescriptorAllocator>,
}

pub struct DestroyCommandPoolTask {
    pub pool: CommandPool,
}

pub trait CleanUpTask<'a> {
    fn execute(&mut self, device: Arc<VkDevice>, malloc: Arc<MemoryAllocator>);
}

impl CleanUpTask<'static> for DestroyImageTask {
    fn execute(&mut self, _device: Arc<VkDevice>, malloc: Arc<MemoryAllocator>) {
        unsafe { malloc.destroy_image(self.image, &mut self.allocation) };
        debug!("Image ({:?}) has been deleted", self.image);
    }
}

impl CleanUpTask<'static> for DestroyCommandPoolTask {
    fn execute(&mut self, device: Arc<VkDevice>, malloc: Arc<MemoryAllocator>) {
        unsafe { device.destroy_command_pool(self.pool, None) }
        debug!("Pool ({:?}) has been deleted", self.pool);
    }
}

impl CleanUpTask<'static> for DestroyDescriptorPools {
    fn execute(&mut self, device: Arc<VkDevice>, malloc: Arc<MemoryAllocator>) {
        debug!("1");
        self.allocator.borrow_mut().destroy_pools(device);
        debug!("DescriptorPool have been deleted");
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
