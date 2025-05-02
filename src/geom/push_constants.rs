use std::marker::PhantomData;

use ash::vk::DeviceMemory;

#[derive(Copy, Clone)]
pub struct PushConstant<T: Sized> {
    push_constant: T,
    address: DeviceMemory,
}

impl<T: Sized> PushConstant<T> {
    pub fn new(push_constant: T, address: DeviceMemory) -> Self {
        Self {
            push_constant,
            address,
        }
    }

    pub fn size(&self) -> usize {
        std::mem::size_of::<T>()
    }

    pub fn raw_data(&self) -> Vec<u8> {
        let data_ptr = &self.push_constant as *const T;
        unsafe { std::slice::from_raw_parts(data_ptr as *const u8, self.size()).to_vec() }
    }
}
