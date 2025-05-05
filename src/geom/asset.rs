use std::{fs::File, io::BufReader, path::Path, primitive, usize};

use anyhow::Result;
use gltf::{buffer, json, mesh::Reader, Accessor, Buffer, Glb, Gltf, Mesh, Semantic};
use image::io;
use log::debug;

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

    pub fn load_gltf_meshes<P: AsRef<Path>>(file_path: P) -> Result<Vec<MeshAsset<Vertex3D>>> {
        let mut mesh_assets: Vec<MeshAsset<Vertex3D>> = vec![];
        let mut vertices: Vec<Vertex3D> = vec![];
        let mut indices: Vec<usize> = vec![];
        let gltf = gltf::Gltf::open(file_path)?;
        let meshes = gltf.meshes();
        let buffers = gltf.buffers().collect::<Vec<Buffer>>();
        for mesh in meshes {
            indices.clear();
            vertices.clear();
            let primitives = mesh.primitives();
            for primitive in primitives {
                let reader = primitive.reader(|buffer| buffers[buffer.index()]);
                
                let surface = GeoSurface {
                    start_index: indices.len() as u32,
                    count: primitive.indices().unwrap().count(),
                };

                let initial_vtx = vertices.len();
                indices.push(index_accessor.index() + initial_vtx);
                {
                    let position_accessor = &gltf_accessors[primitive
                        .attributes()
                        .find(|(semantic, attribute)| semantic.eq(&Semantic::Positions))
                        .unwrap()
                        .1
                        .index()];
                    position_accessor.read
                }
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
