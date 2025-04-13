use std::{ io::{Cursor, Error}, path::Path};

use ash::{util::read_spv, vk::{ShaderModule, ShaderModuleCreateInfo}, Device};


pub fn load_shader_module(file_path: &str, device: &Device) -> Result<ShaderModule, Error> {
    let code: _ = read_spv(&mut read_file_as_cursor(file_path)).unwrap();
    let create_info = ShaderModuleCreateInfo::default().code(&code);
    Ok(unsafe { device.create_shader_module(&create_info, None).unwrap() })
}

pub fn read_file_as_cursor<P: AsRef<Path>>(path: P) -> Cursor<Vec<u8>> {
     Cursor::new(std::fs::read(path).unwrap())
}
