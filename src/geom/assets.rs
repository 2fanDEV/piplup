use std::{fmt::Display, ops::DerefMut, path::Path, sync::{Arc, Mutex}, usize};

use anyhow::{anyhow, Result};
use ash::vk::{Rect2D, Viewport};
use log::debug;
use nalgebra::{Vector2, Vector3, Vector4};

use crate::{components::{
    allocation_types::VkBuffer, command_buffers::VkCommandPool, memory_allocator::MemoryAllocator,
    queue::VkQueue,
}, misc::{material::MaterialInstance, render_object::Node}};

use super::{
    mesh::{self, MeshBuffers},
    vertex_3d::Vertex3D,
    VertexAttributes,
};

#[derive(Clone, Debug, Default)]
pub struct GLTFMaterial {
    pub data: MaterialInstance
}

#[derive(Default, Debug, Clone)]
pub struct GeoSurface {
    pub start_index: u32,
    pub count: usize,
    pub material: Option<Arc<GLTFMaterial>>
}

#[derive(Debug)]
pub struct MeshAsset<T: VertexAttributes> {
    pub name: String,
    pub surfaces: Vec<GeoSurface>,
    pub mesh_buffers: MeshBuffers<T, u32>,
}


impl GeoSurface {
    pub fn material(&mut self, material: Option<Arc<GLTFMaterial>>) -> Self {
        self.material = material;
        self.clone()
    }
}

impl<T: VertexAttributes> MeshAsset<T> {
    pub fn new(name: String, surfaces: Vec<GeoSurface>, mesh_buffers: MeshBuffers<T, u32>) -> Self {
        Self {
            name,
            surfaces,
            mesh_buffers,
        }
    }

    pub fn load_gltf_meshes<P: AsRef<Path> + Display>(
        file_path: P,
        scissors: Rect2D,
        viewport: Viewport,
        memory_allocator: Arc<MemoryAllocator>,
        queues: &[Arc<VkQueue>],
        command_pool: VkCommandPool,
    ) -> Result<Vec<Arc<Mutex<MeshAsset<Vertex3D>>>>> {
        let mut mesh_assets: Vec<Arc<Mutex<MeshAsset<Vertex3D>>>> = vec![];
        let mut vertices: Vec<Vertex3D> = vec![];
        let mut indices: Vec<usize> = vec![];
        let mut surfaces: Vec<GeoSurface> = vec![];
        let mut meshes: Vec<mesh::Mesh<Vertex3D, u32>> = vec![];
        let gltf = gltf::Gltf::open(&file_path)?;
        let gltf_meshes = gltf.meshes();
        let blob = &gltf.blob;
        for mesh in gltf_meshes {
            meshes.clear();
            indices.clear();
            vertices.clear();
            surfaces.clear();
            let primitives = mesh.primitives();
            for primitive in primitives {
                let reader = primitive.reader(|_buffer| blob.as_deref());
                let surface = GeoSurface {
                    start_index: indices.len() as u32,
                    count: primitive.indices().unwrap().count(),
                    material: None
                };
                surfaces.push(surface);
                //let initial_vtx = vertices.len();
                let positions = reader
                    .read_positions()
                    .ok_or(anyhow!("There are no positions in this mesh"))?
                    .collect::<Vec<_>>();
                let normals = reader
                    .read_normals()
                    .ok_or(anyhow!("There are no normals in this mesh"))?
                    .collect::<Vec<_>>();
                let indices = reader
                    .read_indices()
                    .ok_or(anyhow!("There are no indices in this mesh"))?
                    .into_u32()
                    .collect::<Vec<_>>();

                let uvs = reader
                    .read_tex_coords(0)
                    .ok_or(anyhow!("There are uv"))?
                    .into_f32()
                    .collect::<Vec<_>>();
                let colors = match reader.read_colors(0) {
                    Some(colors) => colors.into_rgba_f32().collect::<Vec<_>>(),
                    None => normals
                        .iter()
                        .map(|normal| [normal[0], normal[1], normal[2], 1.0])
                        .collect::<Vec<_>>(),
                };
                let override_color = false;
                let white_color = [1.0, 1.0, 1.0, 1.0];

                for (idx, pos_arr) in positions.into_iter().enumerate() {
                    let pos = Vector3::new(pos_arr[0], pos_arr[1], pos_arr[2]);
                    let normal_arr = normals[idx];
                    let normal = Vector3::new(normal_arr[0], normal_arr[1], normal_arr[2]);
                    let uv_arr = uvs[idx];
                    let color_arr = if override_color { colors[idx] } else { white_color };
                    let color =
                        Vector4::<f32>::new(color_arr[0], color_arr[1], color_arr[2], color_arr[3]);
                    vertices.push(Vertex3D::new(
                        pos,
                        Vector2::new(uv_arr[0] as f32, uv_arr[1] as f32),
                        normal,
                        color,
                    ));
                }

                
                let mesh_buffer = MeshBuffers::new(
                    mesh::Mesh::<Vertex3D, u32> {
                        vertices: vertices.clone(),
                        indices,
                        texture_id: None,
                        scissors,
                        viewport,
                    },
                    |buffer_elements, buffer_usage, memory_usage, memory_property_flags| {
                        memory_allocator
                            .create_buffer_with_mapped_memory(
                                &buffer_elements,
                                queues,
                                buffer_usage,
                                memory_usage,
                                memory_property_flags,
                                &command_pool,
                            )
                            .unwrap()
                            .unit
                            .get_copied::<VkBuffer>()
                    },
                    |buffer_elements, buffer_usage, memory_usage, memory_property_flags| {
                        memory_allocator
                            .create_buffer_with_mapped_memory(
                                &buffer_elements,
                                queues,
                                buffer_usage,
                                memory_usage,
                                memory_property_flags,
                                &command_pool,
                            )
                            .unwrap()
                            .unit
                            .get_copied::<VkBuffer>()
                    },
                )?;
                
                mesh_assets.push(Arc::new(Mutex::new(MeshAsset::new(
                    mesh.name().map(|s| s.to_owned()).unwrap(),
                    surfaces.clone(),
                    mesh_buffer,
                ))));
            }
        }

        Ok(mesh_assets)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn load_gltf_meshes() {
        let file_path = "/Users/zapzap/Projects/piplup/assets/basicmesh.glb";
        //       let mesh_asset = MeshAsset::<Vertex3D>::load_gltf_meshes(file_path);
        //     assert_eq!(mesh_asset.is_ok(), true);
        //   assert_eq!(mesh_asset.unwrap().is_empty(), false);
    }
}
