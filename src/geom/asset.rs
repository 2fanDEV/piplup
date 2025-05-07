use std::{fmt::Display, fs::File, io::BufReader, path::Path, primitive, usize};

use anyhow::{anyhow, Result};
use ash::vk::{Rect2D, Viewport};
use gltf::{
    accessor::{self, util::ItemIter},
    buffer, json,
    mesh::Reader,
    Accessor, Buffer, Glb, Gltf, Mesh, Semantic,
};
use image::io;
use log::debug;
use nalgebra::{Vector3, Vector4};
use ndarray::iter::Iter;

use super::{
    mesh::{self, Mesh, MeshBuffers},
    vertex_3d::Vertex3D,
    VertexAttributes,
};

#[derive(Default, Debug)]
struct GeoSurface {
    start_index: u32,
    count: usize,
}

#[derive(Debug)]
pub struct MeshAsset<T: VertexAttributes> {
    name: String,
    surfaces: Vec<GeoSurface>,
    mesh_buffers: MeshBuffers<T, u32>,
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
    ) -> Result<Vec<MeshAsset<Vertex3D>>> {
        let mut mesh_assets: Vec<MeshAsset<Vertex3D>> = vec![];
        let mut vertices: Vec<Vertex3D> = vec![];
        let mut indices: Vec<usize> = vec![];
        let mut meshes: Vec<Mesh> = vec![];
        let gltf = gltf::Gltf::open(&file_path)?;
        let gltf_meshes = gltf.meshes();
        let blob = &gltf.blob;
        for mesh in gltf_meshes {
            meshes.clear();
            indices.clear();
            vertices.clear();
            let primitives = mesh.primitives();
            for primitive in primitives {
                let reader = primitive.reader(|buffer| blob.as_deref());
                let surface = GeoSurface {
                    start_index: indices.len() as u32,
                    count: primitive.indices().unwrap().count(),
                };

                let initial_vtx = vertices.len();
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
                    .into_u8()
                    .collect::<Vec<_>>();
                let colors = reader
                    .read_colors(0)
                    .ok_or(anyhow!("There are not colors for this mesh"))?
                    .into_rgba_f32()
                    .collect::<Vec<_>>();

                for (idx, pos_arr) in positions.into_iter().enumerate() {
                    let pos = Vector3::new(pos_arr[0], pos_arr[1], pos_arr[2]);
                    let normal_arr = normals[idx];
                    let normal = Vector3::new(normal_arr[0], normal_arr[1], normal_arr[2]);
                    let uv_arr = uvs[idx];
                    let color_arr = colors[idx];
                    let color =
                        Vector4::new(color_arr[0], color_arr[1], color_arr[2], color_arr[3]);
                    vertices.push(Vertex3D {
                        pos,
                        uv_x: uv_arr[0] as f32,
                        normal,
                        uv_y: uv_arr[1] as f32,
                        color,
                    });
                }

                let mesh::Mesh::<Vertex3D, u32> {
                    vertices,
                    indices,
                    texture_id: None,
                    scissors,
                    viewport,
                };
                MeshBuffers::new(meshes, create_vertex_buffer, create_index_buffer);
            }
            mesh_assets.push(MeshAsset::new(
                mesh.name().map(|s| s.to_owned()).unwrap(),
                todo!(),
                todo!(),
            ));
        }

        Ok(mesh_assets)
    }
}

#[cfg(test)]
mod tests {
    use super::MeshAsset;
    use crate::geom::vertex_3d::Vertex3D;

    #[test]
    fn load_gltf_meshes() {
        let file_path = "/Users/zapzap/Projects/piplup/assets/basicmesh.glb";
        let mesh_asset = MeshAsset::<Vertex3D>::load_gltf_meshes(file_path);
        assert_eq!(mesh_asset.is_ok(), true);
        assert_eq!(mesh_asset.unwrap().is_empty(), false);
    }
}
