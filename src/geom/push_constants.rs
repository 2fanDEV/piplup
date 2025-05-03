use ash::vk::DeviceAddress;
use log::debug;

#[derive(Default, Copy, Clone)]
pub struct PushConstant<T: Sized + Default> {
    push_constant: T,
    device_address: DeviceAddress
}

impl<T: Sized> PushConstant<T>
where
    T: Sized + Default,
{
    pub fn new(push_constant: T) -> Self {
        Self { push_constant,
        device_address: DeviceAddress::default()}
    }

    pub fn size(&self) -> usize {
        std::mem::size_of::<T>()
    }

    pub fn raw_data(&self) -> Vec<u8> {
        let data_ptr = &self.push_constant as *const T;
        unsafe { std::slice::from_raw_parts(data_ptr as *const u8, self.size()).to_vec() }
    }
}
