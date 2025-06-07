use std::{
    rc::Weak,
    sync::{Arc, Mutex},
};

use ash::vk::DeviceAddress;
use nalgebra::Matrix4;

use crate::{
    components::allocation_types::VkBuffer,
    geom::{assets::MeshAsset, VertexAttributes},
};

use super::{material::MaterialInstance, RenderNode, Renderable};

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct RenderObject {
    index_count: u32,
    first_index: u32,
    index_buffer: VkBuffer,
    transform: Matrix4<f32>,
    material: MaterialInstance,
    vertex_buffer_address: DeviceAddress,
}

#[allow(unused)]
pub struct Node {
    parent: Weak<Node>,
    children: Vec<Arc<Node>>,
    local_transform: Matrix4<f32>,
    world_transform: Mutex<Matrix4<f32>>, // Wrap world_transform in a Mutex
}

impl RenderNode for Node {}

impl Node {
    pub fn new(
        parent: Weak<Node>,
        children: Vec<Arc<Node>>,
        local_transform: Matrix4<f32>,
        world_transform: Matrix4<f32>,
    ) -> Self {
        Self {
            parent,
            children,
            local_transform,
            world_transform: Mutex::new(world_transform),
        }
    }

    fn refresh_transform(&self, parent_matrix: Matrix4<f32>) {
        // Takes immutable self
        let mut world_transform = self.world_transform.lock().unwrap(); // Lock the Mutex to get mutable access
        *world_transform = parent_matrix * self.local_transform;
        for child in &self.children {
            child.refresh_transform(*world_transform); // Pass the updated world_transform
        }
    }
}

impl Renderable for Node {
    fn draw(&self, top_matrix: Matrix4<f32>, draw_ctx: &mut super::DrawContext) {
        // maybe &self.children instead of clone
        for child in &self.children {
            // Iterate over immutable references
            child.draw(top_matrix, draw_ctx);
        }
    }
}


pub struct MeshNode<T: VertexAttributes> {
    pub node: Arc<Node>,
    pub mesh_asset: Arc<Mutex<MeshAsset<T>>>,
}

impl <T:VertexAttributes> RenderNode for MeshNode<T> {}

impl<T: VertexAttributes> Renderable for MeshNode<T> {
    fn draw(&self, top_matrix: Matrix4<f32>, draw_ctx: &mut super::DrawContext) {
        let node_matrix = top_matrix * *self.node.world_transform.lock().unwrap();
        for surface in &self.mesh_asset.lock().unwrap().surfaces {
            let render_obj = RenderObject {
                index_count: surface.count as u32,
                first_index: surface.start_index,
                index_buffer: self.mesh_asset.lock().unwrap().mesh_buffers.index_buffer,
                material: surface.material.clone().unwrap().data.clone(),
                transform: node_matrix,
                vertex_buffer_address: self.mesh_asset.lock().unwrap().mesh_buffers.vertex_buffer.address,
            };

            draw_ctx.opaque_surfaces.push(render_obj);
        }

        self.node.draw(top_matrix, draw_ctx);
    }
}
