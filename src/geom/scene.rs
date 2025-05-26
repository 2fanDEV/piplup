use nalgebra::{Matrix4, Vector4};

#[derive(Default)]
pub struct SceneData {
    view: Matrix4<f32>,
    proj: Matrix4<f32>,
    view_proj: Matrix4<f32>,
    ambient_color: Vector4<f32>,
    sunlight_direction: Vector4<f32>,
    sunlight_color: Vector4<f32>,
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
