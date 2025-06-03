use crate::components::allocation_types::VkBuffer;

pub struct RenderObject {
    pub start_index: u32,
    pub first_index: u32,
    pub buffer: VkBuffer, 
}
