use nalgebra::{Matrix4, Vector4};

#[repr(C)]
#[derive(Debug, Default)]
pub struct SceneData {
    pub view: Matrix4<f32>,
    pub proj: Matrix4<f32>,
    pub view_proj: Matrix4<f32>,
    pub ambient_color: Vector4<f32>,
    pub sunlight_direction: Vector4<f32>,
    pub sunlight_color: Vector4<f32>,
}

impl SceneData {
    pub fn new(
        view: Matrix4<f32>,
        proj: Matrix4<f32>,
        view_proj: Matrix4<f32>,
        ambient_color: Vector4<f32>,
        sunlight_direction: Vector4<f32>,
        sunlight_color: Vector4<f32>,
    ) -> Self {
        Self {
            view,
            proj,
            view_proj,
            ambient_color,
            sunlight_direction,
            sunlight_color,
        }
    }
}
