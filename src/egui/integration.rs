use std::{io::Error, ops::Deref, sync::Arc};

use ash::vk::{BufferUsageFlags, MemoryPropertyFlags};
use cgmath::{Vector2, Vector4};
use egui::{
    epaint::{Primitive, TextureAtlas}, text::Fonts, ClippedPrimitive, Context, FullOutput, TexturesDelta, ViewportId
};
use egui_winit::{EventResponse, State};
use log::debug;
use winit::{
    event::WindowEvent,
    window::{Theme, Window},
};

use crate::components::{
    buffers::VkBuffer, command_buffers::VkCommandPool, geom::vertex::Vertex2D,
    memory_allocator::MemoryAllocator, queue::VkQueue,
};

pub struct Mesh {
    pub vertices: Vec<Vertex2D>,
    pub indices: Vec<u32>,
    pub texture_id: u64,
}

#[derive(Debug)]
pub struct MeshBuffers {
    pub vertex_buffer: VkBuffer,
    pub indices_buffer: VkBuffer,
    pub indices: Vec<u32>,
    pub vertices: Vec<Vertex2D>,
}

impl MeshBuffers {
    #[allow(deprecated)]
    pub fn new(
        mesh: Mesh,
        allocator: &MemoryAllocator,
        queue: Arc<VkQueue>,
        command_pool: &VkCommandPool,
    ) -> Result<MeshBuffers, Error> {
        let queue = vec![queue];
        let vertex_buffer = allocator.create_buffer(
            &mesh.vertices,
            &queue,
            BufferUsageFlags::VERTEX_BUFFER,
            vk_mem::MemoryUsage::GpuOnly,
            MemoryPropertyFlags::DEVICE_LOCAL,
            command_pool,
        )?;
        let indices_buffer = allocator.create_buffer(
            &mesh.indices,
            &queue,
            BufferUsageFlags::INDEX_BUFFER,
            vk_mem::MemoryUsage::GpuOnly,
            MemoryPropertyFlags::DEVICE_LOCAL,
            command_pool,
        )?;
        Ok(Self {
            vertex_buffer,
            indices_buffer,
            vertices: mesh.vertices,
            indices: mesh.indices,
        })
    }
}

pub struct EguiIntegration {
    state: State,
    has_run: bool,
}

impl EguiIntegration {
    pub fn new(window: &Window) -> Self {
        let state = State::new(
            Context::default(),
            ViewportId::ROOT,
            window,
            Some(2.0 * window.scale_factor() as f32),
            Some(Theme::Dark),
            Some(1024 * 4),
        );

        Self { state, has_run: false }
    }

    pub fn input(&mut self, window: &Window, event: &WindowEvent) -> EventResponse {
        self.state.on_window_event(window, event)
    }

    pub fn run(&mut self, run_ui: impl FnMut(&Context), window: &Window) -> FullOutput {
        let raw_input = self.state.take_egui_input(window);
        let output = self.state.egui_ctx().run(raw_input.clone(), run_ui);
        self.has_run = true;
        self.state
            .handle_platform_output(window, output.platform_output.clone());
        output
    }

    pub fn get_fonts(&mut self) -> Option<Fonts> {
        if self.has_run {   
            Some(self.state.egui_ctx().fonts(|reader| reader.clone()))
        } else {
            None
        }
    }
    
    #[allow(unused)]
    pub fn convert(&mut self, output: FullOutput) -> Vec<Mesh> {
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
                                    vertex.color.r(),
                                    vertex.color.g(),
                                    vertex.color.b(),
                                    vertex.color.a(),
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
