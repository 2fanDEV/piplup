use std::{io::Error, sync::Arc};

use ash::vk::{BufferUsageFlags, MemoryPropertyFlags};
use cgmath::{Vector2, Vector4};
use egui::{epaint::Primitive, ClippedPrimitive, Context, FullOutput, RawInput, Rect, ViewportId};
use egui_winit::State;
use log::debug;
use vk_mem::Allocator;
use winit::window::{self, Window};

use crate::components::{
    buffers::VkBuffer, command_buffers::VkCommandPool, geom::vertex::Vertex2D, queue::VkQueue,
};

pub struct Mesh {
    pub vertices: Vec<Vertex2D>,
    pub indices: Vec<u32>,
    pub texture_id: u64,
}

pub struct MeshBuffers {
    pub vertex_buffer: VkBuffer,
    pub indices_buffer: VkBuffer,
}

impl MeshBuffers {
    pub fn new(
        mesh: Mesh,
        allocator: Allocator,
        queue: VkQueue,
        command_pool: Arc<VkCommandPool>,
    ) -> Result<MeshBuffers, Error> {
        let queue = vec![queue];
        let vertex_buffer = VkBuffer::create_buffer(
            &allocator,
            &mesh.vertices,
            &queue,
            BufferUsageFlags::VERTEX_BUFFER,
            vk_mem::MemoryUsage::GpuOnly,
            MemoryPropertyFlags::DEVICE_LOCAL,
            command_pool.clone(),
        )?;
        let indices_buffer = VkBuffer::create_buffer(
            &allocator,
            &mesh.vertices,
            &queue,
            BufferUsageFlags::VERTEX_BUFFER,
            vk_mem::MemoryUsage::GpuOnly,
            MemoryPropertyFlags::DEVICE_LOCAL,
            command_pool.clone()
        )?;
        Ok(Self { vertex_buffer, indices_buffer })
    }
}

pub struct EguiIntegration {
    state: State,
}

impl EguiIntegration {
    pub fn new(window: &Window) -> Self {
        let state = State::new(
            Context::default(),
            ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
            Some(1024 * 2),
        );
        Self { state }
    }

    pub fn run(&mut self, run_ui: impl FnMut(&Context), window: &Window) -> Vec<Mesh> {
        let raw_input = self.state.take_egui_input(window);
        debug!("{raw_input:?}");
        let output = self.state.egui_ctx().run(raw_input, run_ui);
        self.convert(output)
    }

    fn convert(&self, output: FullOutput) -> Vec<Mesh> {
        let clipped_primitives = self
            .state
            .egui_ctx()
            .tessellate(output.shapes, self.state.egui_ctx().pixels_per_point());

        let mut meshes: Vec<Mesh> = Vec::new();

        for ClippedPrimitive {
            primitive,
            clip_rect,
        } in clipped_primitives
        {
            match primitive {
                Primitive::Mesh(mesh) => {
                    let indices = mesh.indices;
                    let vertices = mesh
                        .vertices
                        .iter()
                        .map(|vertex| {
                            Vertex2D::new(
                                Vector2::new(vertex.pos.x, vertex.pos.y),
                                Vector4::new(
                                    vertex.color.r() as f32,
                                    vertex.color.g() as f32,
                                    vertex.color.b() as f32,
                                    vertex.color.a() as f32,
                                ),
                                Vector2::new(vertex.uv.x, vertex.uv.y),
                            )
                        })
                        .collect::<Vec<Vertex2D>>();
                    let texture_id = match mesh.texture_id {
                        egui::TextureId::Managed(id) => id,
                        egui::TextureId::User(id) => id,
                    };
                    meshes.push(Mesh {
                        vertices,
                        indices,
                        texture_id,
                    });
                }
                Primitive::Callback(paint_callback) => todo!(),
            }
        }
        meshes
    }
}
