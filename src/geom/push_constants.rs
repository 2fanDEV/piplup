
#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct PushConstant<T: Sized + Default> {
    push_constant: T,
    device_address: u64,
    _padding: [u8; 8], // <--- Add 8 bytes of explicit padding here
}

impl<T: Sized> PushConstant<T>
where
    T: Sized + Default
{
    pub fn new(push_constant: T, device_address: u64) -> Self {
        Self {
            push_constant,
            device_address,
            ..Default::default()
        }
    }

    pub fn size(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    #[allow(non_snake_case)]
    pub fn size_of_T(&self) -> usize {
        std::mem::size_of::<T>()
    }

    #[allow(non_snake_case)]
    pub fn raw_data_of_T(&self) -> Vec<u8> {
        let data_ptr = &self.push_constant as *const T;
        unsafe { std::slice::from_raw_parts(data_ptr as *const u8, self.size_of_T()).to_vec() }
    }

    pub fn raw_data(&self) -> Vec<u8> {
        let data_ptr: *const Self = self;
        unsafe { std::slice::from_raw_parts(data_ptr as *const u8, self.size()).to_vec() }
    }
}
