use nalgebra::{Matrix4, Quaternion, Unit, UnitQuaternion, Vector2, Vector3, Vector4};
use winit::{
    event::{KeyEvent, WindowEvent},
    keyboard::KeyCode,
};

pub struct Camera {
    velocity: Vector3<f32>,
    position: Vector3<f32>,
    pitch: f32,
    yaw: f32,
}

impl Camera {
    pub fn update(&mut self) {
        let camera_rotation = self.get_rotation_matrix();
        let velocity = self.velocity * 0.5;
        let position_multiplier =
            camera_rotation * Vector4::new(velocity[0], velocity[1], velocity[2], 0.0);
        self.position = self.position
            + Vector3::new(
                position_multiplier[0],
                position_multiplier[1],
                position_multiplier[2],
            );
    }

    pub fn get_view_matrix(&self) -> Matrix4<f32> {
        let translation = Matrix4::new_translation(&self.position);
        let camera_rotation = self.get_rotation_matrix();
        let matrix = camera_rotation * translation;
        matrix.try_inverse().unwrap()
    }

    pub fn get_rotation_matrix(&self) -> Matrix4<f32> {
        let pitch_rotation =
            UnitQuaternion::from_axis_angle(&Vector3::x_axis(), self.pitch);
        let yaw_rotation =
            UnitQuaternion::from_axis_angle(&Vector3::y_axis(), self.yaw);
        let x = yaw_rotation * pitch_rotation;
        x.to_homogeneous()
    }

    pub fn process_events(&mut self, window_event: WindowEvent) {
        match window_event {
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => match event.physical_key {
                winit::keyboard::PhysicalKey::Code(key_code) => match key_code {
                    KeyCode::KeyW => self.velocity.z = -1.0,
                    KeyCode::KeyS => self.velocity.z = 1.0,
                    KeyCode::KeyA => self.velocity.x = -1.0,
                    KeyCode::KeyD => self.velocity.x = 1.0,
                    _ => return,
                },
                winit::keyboard::PhysicalKey::Unidentified(native_key_code) => return,
            },
            WindowEvent::AxisMotion {
                device_id,
                axis,
                value,
            } => {
                return; //TODO yaw and pitch
            }
            _ => return,
        }
    }
}
