use std::{collections::HashMap, env::join_paths, io::Error, ops::Deref, sync::Arc};

use ash::vk::{BufferUsageFlags, MemoryPropertyFlags, Viewport};
use cgmath::{Vector2, Vector4};
use egui::{
    epaint::Primitive, ClippedPrimitive, Context, FullOutput, Id, Pos2, RawInput, Rect, Vec2,
    ViewportId, ViewportInfo,
};
use egui_winit::{EventResponse, State};
use vk_mem::Allocator;
use winit::{event::WindowEvent, window::{self, Theme, Window}};

use crate::components::{
    buffers::VkBuffer, command_buffers::VkCommandPool, geom::vertex::Vertex2D, memory_allocator::MemoryAllocator, queue::VkQueue
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
    pub vertices: Vec<Vertex2D>
}

impl MeshBuffers {
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
            indices: mesh.indices
        })
    }

    pub fn remap_contents(mesh: Mesh) {
        
    }
}

pub struct EguiIntegration {
    state: State
}

impl EguiIntegration {
    pub fn new(window: &Window) -> Self {
        let state = State::new(
            Context::default(),
            ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            Some(Theme::Dark),
            Some(1024 * 2),
        );

        Self { state }
    }

    pub fn input(&mut self, window: &Window, event: &WindowEvent) -> EventResponse {
        self.state.on_window_event(window, event)
    }

    pub fn run(&mut self, run_ui: impl FnMut(&Context), window: &Window) -> Vec<Mesh> {
        let mut raw_input = self.state.take_egui_input(window);
        let window_size = window.inner_size();
      /*  let egui_rect = Rect::from_min_size(
            Pos2::new(0.0, 0.0),
            Vec2::new((window_size.width - 100) as f32, (window_size.height - 100) as f32),
        );

        raw_input.screen_rect = Some(egui_rect);

        // Create or update the viewport info:
        let viewport_info = ViewportInfo {
            inner_rect: Some(egui_rect),
            outer_rect: Some(egui_rect),
            ..Default::default() // Fill in other fields with defaults
        };

        if let Some(viewport) = raw_input.viewports.get_mut(&raw_input.viewport_id) {
            viewport.inner_rect = Some(egui_rect);
            viewport.outer_rect = Some(egui_rect);
        } else {
            //handle the error, as the viewport id should exist.
            println!("Error: ViewportId FFFF was not found in the raw_input.viewports hashmap.");
        }
*/
        let output = self.state.egui_ctx().run(raw_input.clone(), run_ui);
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
