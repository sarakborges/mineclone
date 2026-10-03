use std::f32::consts::PI;

use bevy::{
    asset::{AssetId, RenderAssetUsages},
    mesh::Indices,
    pbr::{ExtendedMaterial, MaterialExtension},
    platform::collections::HashMap,
    prelude::*,
    reflect::TypePath,
    render::{render_resource::{AsBindGroup, PrimitiveTopology}, storage::ShaderBuffer},
    shader::ShaderRef,
};

use crate::content::object::ObjectCuboidPartDefinition;

use super::{
    color::{MATERIAL_TINT_RGB_LEVELS, quantize_srgba},
    terrain_material::TerrainLightingBuffer,
};

const OBJECT_PRIMITIVE_VERTEX_SHADER_PATH: &str = "shaders/object_primitive_vertex.wgsl";

pub(crate) type ObjectPrimitiveMaterial =
    ExtendedMaterial<StandardMaterial, ObjectPrimitiveMaterialExtension>;

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub(crate) struct ObjectPrimitiveMaterialExtension {
    #[storage(100, read_only)]
    lighting: Handle<ShaderBuffer>,
    #[uniform(101)]
    wind_sway: f32,
}

impl MaterialExtension for ObjectPrimitiveMaterialExtension {
    fn vertex_shader() -> ShaderRef {
        OBJECT_PRIMITIVE_VERTEX_SHADER_PATH.into()
    }
}

pub(crate) struct ObjectPrimitivesPlugin;

impl Plugin for ObjectPrimitivesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<ObjectPrimitiveMaterial>::default())
            .init_resource::<ObjectPrimitiveMaterialCache>();
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct ObjectPrimitiveMaterialKey {
    texture: AssetId<Image>,
    tint: [u8; 4],
    unlit: bool,
    alpha_cutoff: u32,
    double_sided: bool,
    wind_sway: bool,
}

#[derive(Resource, Default)]
pub(crate) struct ObjectPrimitiveMaterialCache(
    HashMap<ObjectPrimitiveMaterialKey, Handle<ObjectPrimitiveMaterial>>,
);

pub(crate) struct ObjectPrimitiveMaterialRequest {
    pub(crate) texture: Handle<Image>,
    pub(crate) tint: Color,
    pub(crate) unlit: bool,
    pub(crate) alpha_cutoff: f32,
    pub(crate) double_sided: bool,
    pub(crate) wind_sway: bool,
}

pub(crate) struct ObjectPrimitiveMaterialContext<'a> {
    pub(crate) materials: &'a mut Assets<ObjectPrimitiveMaterial>,
    pub(crate) cache: &'a mut ObjectPrimitiveMaterialCache,
    pub(crate) lighting: &'a TerrainLightingBuffer,
}

pub(crate) fn resolve_object_primitive_material(
    request: ObjectPrimitiveMaterialRequest,
    context: ObjectPrimitiveMaterialContext<'_>,
) -> Handle<ObjectPrimitiveMaterial> {
    let (tint, tint_key) = quantize_srgba(request.tint, MATERIAL_TINT_RGB_LEVELS);
    let key = ObjectPrimitiveMaterialKey {
        texture: request.texture.id(),
        tint: tint_key,
        unlit: request.unlit,
        alpha_cutoff: request.alpha_cutoff.to_bits(),
        double_sided: request.double_sided,
        wind_sway: request.wind_sway,
    };
    if let Some(existing) = context.cache.0.get(&key) {
        return existing.clone();
    }

    let mut base = StandardMaterial {
        base_color: tint,
        base_color_texture: Some(request.texture),
        alpha_mode: AlphaMode::Mask(request.alpha_cutoff),
        perceptual_roughness: 1.0,
        unlit: request.unlit,
        double_sided: request.double_sided,
        ..default()
    };
    if request.double_sided {
        base.cull_mode = None;
    }
    let material = context.materials.add(ExtendedMaterial {
        base,
        extension: ObjectPrimitiveMaterialExtension {
            lighting: context.lighting.handle(),
            wind_sway: request.wind_sway as u8 as f32,
        },
    });
    context.cache.0.insert(key, material.clone());
    material
}

pub(crate) fn crossed_sprite_mesh(width: f32, height: f32, base_offset: f32, planes: u8) -> Mesh {
    let mut buffers = PrimitiveMeshBuffers::with_quad_capacity(usize::from(planes) * 2);
    let half_width = width * 0.5;
    let top = base_offset + height;

    for plane in 0..planes {
        let angle = PI * f32::from(plane) / f32::from(planes);
        let axis = Vec3::new(angle.cos(), 0.0, angle.sin()) * half_width;
        let normal = Vec3::new(-angle.sin(), 0.0, angle.cos());
        let bottom_left = Vec3::new(-axis.x, base_offset, -axis.z);
        let bottom_right = Vec3::new(axis.x, base_offset, axis.z);
        let top_right = Vec3::new(axis.x, top, axis.z);
        let top_left = Vec3::new(-axis.x, top, -axis.z);
        let front_uvs = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

        buffers.push_quad(
            [bottom_left, bottom_right, top_right, top_left],
            normal,
            front_uvs,
        );
        buffers.push_quad(
            [bottom_right, bottom_left, top_left, top_right],
            -normal,
            [[1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]],
        );
    }

    buffers.finish()
}

pub(crate) fn cuboid_set_mesh(parts: &[ObjectCuboidPartDefinition]) -> Mesh {
    let mut buffers = PrimitiveMeshBuffers::with_quad_capacity(parts.len() * 6);
    for part in parts {
        append_cuboid(&mut buffers, part);
    }
    buffers.finish()
}

fn append_cuboid(buffers: &mut PrimitiveMeshBuffers, part: &ObjectCuboidPartDefinition) {
    let [x0, y0, z0] = part.from;
    let [x1, y1, z1] = part.to;
    let [u0, v0, u1, v1] = part.uv;
    let uvs = [[u0, v1], [u1, v1], [u1, v0], [u0, v0]];

    buffers.push_quad(
        [
            Vec3::new(x0, y0, z1),
            Vec3::new(x0, y0, z0),
            Vec3::new(x0, y1, z0),
            Vec3::new(x0, y1, z1),
        ],
        Vec3::NEG_X,
        uvs,
    );
    buffers.push_quad(
        [
            Vec3::new(x1, y0, z0),
            Vec3::new(x1, y0, z1),
            Vec3::new(x1, y1, z1),
            Vec3::new(x1, y1, z0),
        ],
        Vec3::X,
        uvs,
    );
    buffers.push_quad(
        [
            Vec3::new(x0, y1, z0),
            Vec3::new(x1, y1, z0),
            Vec3::new(x1, y1, z1),
            Vec3::new(x0, y1, z1),
        ],
        Vec3::Y,
        uvs,
    );
    buffers.push_quad(
        [
            Vec3::new(x0, y0, z1),
            Vec3::new(x1, y0, z1),
            Vec3::new(x1, y0, z0),
            Vec3::new(x0, y0, z0),
        ],
        Vec3::NEG_Y,
        uvs,
    );
    buffers.push_quad(
        [
            Vec3::new(x1, y0, z1),
            Vec3::new(x0, y0, z1),
            Vec3::new(x0, y1, z1),
            Vec3::new(x1, y1, z1),
        ],
        Vec3::Z,
        uvs,
    );
    buffers.push_quad(
        [
            Vec3::new(x0, y0, z0),
            Vec3::new(x1, y0, z0),
            Vec3::new(x1, y1, z0),
            Vec3::new(x0, y1, z0),
        ],
        Vec3::NEG_Z,
        uvs,
    );
}

struct PrimitiveMeshBuffers {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl PrimitiveMeshBuffers {
    fn with_quad_capacity(quad_count: usize) -> Self {
        Self {
            positions: Vec::with_capacity(quad_count * 4),
            normals: Vec::with_capacity(quad_count * 4),
            uvs: Vec::with_capacity(quad_count * 4),
            indices: Vec::with_capacity(quad_count * 6),
        }
    }

    fn push_quad(&mut self, positions: [Vec3; 4], normal: Vec3, uvs: [[f32; 2]; 4]) {
        let base = u32::try_from(self.positions.len())
            .expect("bounded object primitive mesh must fit u32 indices");
        self.positions
            .extend(positions.into_iter().map(|position| position.to_array()));
        self.normals.extend_from_slice(&[normal.to_array(); 4]);
        self.uvs.extend_from_slice(&uvs);
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn finish(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_indices(Indices::U32(self.indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossed_sprite_builds_two_sides_per_plane() {
        let mesh = crossed_sprite_mesh(0.75, 0.6, 0.0, 3);
        assert_eq!(mesh.count_vertices(), 24);
    }

    #[test]
    fn cuboid_set_builds_six_faces_per_part() {
        let part = ObjectCuboidPartDefinition {
            from: [-0.1, 0.0, -0.1],
            to: [0.1, 0.6, 0.1],
            uv: [0.0, 0.0, 1.0, 1.0],
        };
        let mesh = cuboid_set_mesh(&[part]);
        assert_eq!(mesh.count_vertices(), 24);
    }
}
