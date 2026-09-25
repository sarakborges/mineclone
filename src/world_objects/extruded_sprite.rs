use bevy::{
    asset::{AssetId, RenderAssetUsages},
    ecs::system::SystemParam,
    mesh::Indices,
    platform::collections::HashMap,
    prelude::*,
    render::render_resource::PrimitiveTopology,
};

use super::quantized_object_tint;

const MAX_EXTRUDED_SPRITE_PIXELS: u64 = 65_536;
const MAX_EXTRUDED_SPRITE_QUADS: usize = 131_072;

#[derive(Clone, Copy)]
pub(super) struct ExtrudedSpriteGeometry {
    pub(super) size: [f32; 2],
    pub(super) height: f32,
    pub(super) base_offset: f32,
    pub(super) alpha_cutoff: f32,
}

#[derive(Component)]
pub(super) struct PendingExtrudedSprite {
    texture: Handle<Image>,
    geometry: ExtrudedSpriteGeometry,
    tint: Color,
    unlit: bool,
}

impl PendingExtrudedSprite {
    pub(super) fn new(
        texture: Handle<Image>,
        geometry: ExtrudedSpriteGeometry,
        tint: Color,
        unlit: bool,
    ) -> Self {
        Self {
            texture,
            geometry,
            tint,
            unlit,
        }
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct ExtrudedSpriteMeshKey {
    texture: AssetId<Image>,
    width: u32,
    depth: u32,
    height: u32,
    base_offset: u32,
    alpha_cutoff: u32,
}

#[derive(Resource, Default)]
pub(crate) struct ExtrudedSpriteMeshCache(HashMap<ExtrudedSpriteMeshKey, Handle<Mesh>>);

impl ExtrudedSpriteMeshCache {
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct ExtrudedSpriteMaterialKey {
    texture: AssetId<Image>,
    tint: [u8; 4],
    unlit: bool,
    alpha_cutoff: u32,
}

#[derive(Resource, Default)]
pub(crate) struct ExtrudedSpriteMaterialCache(
    HashMap<ExtrudedSpriteMaterialKey, Handle<StandardMaterial>>,
);

impl ExtrudedSpriteMaterialCache {
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(SystemParam)]
pub(super) struct ExtrudedSpriteAssets<'w> {
    images: Res<'w, Assets<Image>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    mesh_cache: ResMut<'w, ExtrudedSpriteMeshCache>,
    material_cache: ResMut<'w, ExtrudedSpriteMaterialCache>,
}

pub(super) fn configure_pending_extruded_sprites(
    mut commands: Commands,
    pending: Query<(Entity, &PendingExtrudedSprite)>,
    mut assets: ExtrudedSpriteAssets,
) {
    for (entity, pending) in &pending {
        let key = mesh_key(pending);
        let cached = assets.mesh_cache.0.get(&key).cloned();
        let mesh = if let Some(mesh) = cached {
            mesh
        } else {
            let Some(image) = assets.images.get(&pending.texture) else {
                continue;
            };
            let generated = match extruded_sprite_mesh(image, pending.geometry) {
                Ok(mesh) => mesh,
                Err(error) => {
                    warn!(
                        "cannot build extruded sprite {:?}: {error}",
                        pending.texture.id()
                    );
                    commands.entity(entity).remove::<PendingExtrudedSprite>();
                    continue;
                }
            };
            let mesh = assets.meshes.add(generated);
            assets.mesh_cache.0.insert(key, mesh.clone());
            mesh
        };

        let (tint, tint_key) = quantized_object_tint(pending.tint);
        let material_key = ExtrudedSpriteMaterialKey {
            texture: pending.texture.id(),
            tint: tint_key,
            unlit: pending.unlit,
            alpha_cutoff: pending.geometry.alpha_cutoff.to_bits(),
        };
        let material = if let Some(material) = assets.material_cache.0.get(&material_key) {
            material.clone()
        } else {
            let material = assets.materials.add(StandardMaterial {
                base_color: tint,
                base_color_texture: Some(pending.texture.clone()),
                alpha_mode: AlphaMode::Mask(pending.geometry.alpha_cutoff),
                perceptual_roughness: 1.0,
                unlit: pending.unlit,
                double_sided: true,
                cull_mode: None,
                ..default()
            });
            assets
                .material_cache
                .0
                .insert(material_key, material.clone());
            material
        };

        commands
            .entity(entity)
            .insert((Mesh3d(mesh), MeshMaterial3d(material)))
            .remove::<PendingExtrudedSprite>();
    }
}

fn mesh_key(pending: &PendingExtrudedSprite) -> ExtrudedSpriteMeshKey {
    let geometry = pending.geometry;
    ExtrudedSpriteMeshKey {
        texture: pending.texture.id(),
        width: geometry.size[0].to_bits(),
        depth: geometry.size[1].to_bits(),
        height: geometry.height.to_bits(),
        base_offset: geometry.base_offset.to_bits(),
        alpha_cutoff: geometry.alpha_cutoff.to_bits(),
    }
}

struct OpaqueMask {
    pixels: Vec<bool>,
    width: usize,
    height: usize,
}

impl OpaqueMask {
    fn from_image(image: &Image, alpha_cutoff: f32) -> Result<Self, String> {
        let width = image.texture_descriptor.size.width;
        let height = image.texture_descriptor.size.height;
        if width == 0 || height == 0 {
            return Err("texture has zero width or height".to_owned());
        }

        let pixel_count = u64::from(width) * u64::from(height);
        if pixel_count > MAX_EXTRUDED_SPRITE_PIXELS {
            return Err(format!(
                "texture has {pixel_count} pixels; maximum is {MAX_EXTRUDED_SPRITE_PIXELS}"
            ));
        }

        let width = usize::try_from(width).expect("u32 texture width must fit usize");
        let height = usize::try_from(height).expect("u32 texture height must fit usize");
        let mut pixels = vec![false; width * height];

        for y in 0..height {
            for x in 0..width {
                let color = image
                    .get_color_at(x as u32, y as u32)
                    .map_err(|error| format!("cannot read texture pixels: {error}"))?;
                pixels[y * width + x] = color.to_srgba().alpha >= alpha_cutoff;
            }
        }

        Ok(Self {
            pixels,
            width,
            height,
        })
    }

    fn is_opaque(&self, x: isize, y: isize) -> bool {
        if x < 0 || y < 0 || x >= self.width as isize || y >= self.height as isize {
            return false;
        }
        self.pixels[y as usize * self.width + x as usize]
    }

    fn exposed_edge_count(&self) -> usize {
        let mut count = 0;
        for y in 0..self.height {
            for x in 0..self.width {
                if !self.is_opaque(x as isize, y as isize) {
                    continue;
                }
                count += [
                    (x as isize - 1, y as isize),
                    (x as isize + 1, y as isize),
                    (x as isize, y as isize - 1),
                    (x as isize, y as isize + 1),
                ]
                .into_iter()
                .filter(|&(neighbor_x, neighbor_y)| !self.is_opaque(neighbor_x, neighbor_y))
                .count();
            }
        }
        count
    }
}

struct MeshBuffers {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl MeshBuffers {
    fn with_quad_capacity(quad_count: usize) -> Self {
        Self {
            positions: Vec::with_capacity(quad_count * 4),
            normals: Vec::with_capacity(quad_count * 4),
            uvs: Vec::with_capacity(quad_count * 4),
            indices: Vec::with_capacity(quad_count * 6),
        }
    }

    fn push_quad(
        &mut self,
        positions: [[f32; 3]; 4],
        normal: [f32; 3],
        uvs: [[f32; 2]; 4],
    ) {
        let base = u32::try_from(self.positions.len())
            .expect("bounded extruded sprite mesh must fit u32 indices");
        self.positions.extend_from_slice(&positions);
        self.normals.extend_from_slice(&[normal; 4]);
        self.uvs.extend_from_slice(&uvs);
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

struct SpriteExtrusion<'a> {
    mask: &'a OpaqueMask,
    geometry: ExtrudedSpriteGeometry,
}

impl SpriteExtrusion<'_> {
    fn append_pixel_sides(&self, buffers: &mut MeshBuffers, pixel_x: usize, pixel_y: usize) {
        let u0 = pixel_x as f32 / self.mask.width as f32;
        let u1 = (pixel_x + 1) as f32 / self.mask.width as f32;
        let v0 = pixel_y as f32 / self.mask.height as f32;
        let v1 = (pixel_y + 1) as f32 / self.mask.height as f32;

        let x0 = -self.geometry.size[0] * 0.5 + self.geometry.size[0] * u0;
        let x1 = -self.geometry.size[0] * 0.5 + self.geometry.size[0] * u1;
        let z0 = self.geometry.size[1] * 0.5 - self.geometry.size[1] * v0;
        let z1 = self.geometry.size[1] * 0.5 - self.geometry.size[1] * v1;
        let y0 = self.geometry.base_offset;
        let y1 = self.geometry.base_offset + self.geometry.height;
        let side_uvs = [[u0, v1], [u1, v1], [u1, v0], [u0, v0]];

        if !self.mask.is_opaque(pixel_x as isize - 1, pixel_y as isize) {
            buffers.push_quad(
                [[x0, y0, z1], [x0, y0, z0], [x0, y1, z0], [x0, y1, z1]],
                [-1.0, 0.0, 0.0],
                side_uvs,
            );
        }
        if !self.mask.is_opaque(pixel_x as isize + 1, pixel_y as isize) {
            buffers.push_quad(
                [[x1, y0, z0], [x1, y0, z1], [x1, y1, z1], [x1, y1, z0]],
                [1.0, 0.0, 0.0],
                side_uvs,
            );
        }
        if !self.mask.is_opaque(pixel_x as isize, pixel_y as isize - 1) {
            buffers.push_quad(
                [[x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0]],
                [0.0, 0.0, 1.0],
                side_uvs,
            );
        }
        if !self.mask.is_opaque(pixel_x as isize, pixel_y as isize + 1) {
            buffers.push_quad(
                [[x1, y0, z1], [x0, y0, z1], [x0, y1, z1], [x1, y1, z1]],
                [0.0, 0.0, -1.0],
                side_uvs,
            );
        }
    }
}

fn extruded_sprite_mesh(
    image: &Image,
    geometry: ExtrudedSpriteGeometry,
) -> Result<Mesh, String> {
    let mask = OpaqueMask::from_image(image, geometry.alpha_cutoff)?;
    let quad_count = 2 + mask.exposed_edge_count();
    if quad_count > MAX_EXTRUDED_SPRITE_QUADS {
        return Err(format!(
            "extrusion would create {quad_count} quads; maximum is {MAX_EXTRUDED_SPRITE_QUADS}"
        ));
    }

    let half_x = geometry.size[0] * 0.5;
    let half_z = geometry.size[1] * 0.5;
    let top_y = geometry.base_offset + geometry.height;
    let mut buffers = MeshBuffers::with_quad_capacity(quad_count);

    buffers.push_quad(
        [
            [-half_x, top_y, -half_z],
            [half_x, top_y, -half_z],
            [half_x, top_y, half_z],
            [-half_x, top_y, half_z],
        ],
        [0.0, 1.0, 0.0],
        [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
    );
    buffers.push_quad(
        [
            [-half_x, geometry.base_offset, half_z],
            [half_x, geometry.base_offset, half_z],
            [half_x, geometry.base_offset, -half_z],
            [-half_x, geometry.base_offset, -half_z],
        ],
        [0.0, -1.0, 0.0],
        [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );

    let extrusion = SpriteExtrusion {
        mask: &mask,
        geometry,
    };
    for y in 0..mask.height {
        for x in 0..mask.width {
            if mask.is_opaque(x as isize, y as isize) {
                extrusion.append_pixel_sides(&mut buffers, x, y);
            }
        }
    }

    Ok(Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_indices(Indices::U32(buffers.indices))
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, buffers.positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, buffers.normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, buffers.uvs))
}

#[cfg(test)]
mod tests {
    use super::OpaqueMask;

    #[test]
    fn contour_edges_ignore_internal_pixel_boundaries() {
        let mask = OpaqueMask {
            pixels: vec![
                true, true, false,
                true, true, false,
                false, false, false,
            ],
            width: 3,
            height: 3,
        };

        assert_eq!(mask.exposed_edge_count(), 8);
    }
}
