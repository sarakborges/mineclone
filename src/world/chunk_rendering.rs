use std::collections::HashMap;

use bevy::{light::NotShadowCaster, prelude::*};

use crate::{
    app::game_state::GameState,
    content::{
        biome::BiomeRegistry,
        block::{BlockDefinition, BlockRegistry},
        fluid::{FluidId, FluidRegistry},
    },
    rendering::terrain_material::{TerrainMaterial, TerrainMaterialExtension},
    voxel::{
        chunk::{CHUNK_SIZE, VoxelChunk},
        fluid_mesh::build_fluid_meshes,
        mesh::{BlockFace, build_chunk_mesh},
        world::VoxelWorld,
    },
};

use super::biome_field::BiomeField;

const GRASS_BLOCK_ID: &str = "mineclone:grass";
const CHUNK_NEIGHBORS: [IVec3; 6] = [
    IVec3::X,
    IVec3::NEG_X,
    IVec3::Y,
    IVec3::NEG_Y,
    IVec3::Z,
    IVec3::NEG_Z,
];

#[derive(Clone)]
struct BlockTerrainMaterials {
    top: Handle<TerrainMaterial>,
    bottom: Handle<TerrainMaterial>,
    left: Handle<TerrainMaterial>,
    right: Handle<TerrainMaterial>,
    front: Handle<TerrainMaterial>,
    back: Handle<TerrainMaterial>,
}

#[derive(Resource, Clone)]
pub struct TerrainMaterials {
    blocks: HashMap<String, BlockTerrainMaterials>,
}

impl TerrainMaterials {
    pub fn from_registry(
        blocks: &BlockRegistry,
        asset_server: &AssetServer,
        materials: &mut Assets<TerrainMaterial>,
        roughness: f32,
        metallic: f32,
    ) -> Self {
        let blocks = blocks
            .iter()
            .map(|definition| {
                let block_materials = BlockTerrainMaterials {
                    top: create_material(
                        definition,
                        &definition.textures.top,
                        asset_server,
                        materials,
                        roughness,
                        metallic,
                    ),
                    bottom: create_material(
                        definition,
                        &definition.textures.bottom,
                        asset_server,
                        materials,
                        roughness,
                        metallic,
                    ),
                    left: create_material(
                        definition,
                        &definition.textures.left,
                        asset_server,
                        materials,
                        roughness,
                        metallic,
                    ),
                    right: create_material(
                        definition,
                        &definition.textures.right,
                        asset_server,
                        materials,
                        roughness,
                        metallic,
                    ),
                    front: create_material(
                        definition,
                        &definition.textures.front,
                        asset_server,
                        materials,
                        roughness,
                        metallic,
                    ),
                    back: create_material(
                        definition,
                        &definition.textures.back,
                        asset_server,
                        materials,
                        roughness,
                        metallic,
                    ),
                };

                (definition.id.clone(), block_materials)
            })
            .collect();

        Self { blocks }
    }

    fn for_face(&self, block_id: &str, face: BlockFace) -> &Handle<TerrainMaterial> {
        let block = self
            .blocks
            .get(block_id)
            .unwrap_or_else(|| panic!("missing terrain materials for block: {block_id}"));

        match face {
            BlockFace::Right => &block.right,
            BlockFace::Left => &block.left,
            BlockFace::Top => &block.top,
            BlockFace::Bottom => &block.bottom,
            BlockFace::Front => &block.front,
            BlockFace::Back => &block.back,
        }
    }
}

fn create_material(
    _definition: &BlockDefinition,
    texture: &str,
    asset_server: &AssetServer,
    materials: &mut Assets<TerrainMaterial>,
    roughness: f32,
    metallic: f32,
) -> Handle<TerrainMaterial> {
    materials.add(TerrainMaterial {
        base: StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(asset_server.load(texture.to_owned())),
            perceptual_roughness: roughness,
            metallic,
            ..default()
        },
        extension: TerrainMaterialExtension::default(),
    })
}

#[derive(Resource, Clone)]
pub struct FluidMaterials {
    materials: HashMap<FluidId, Handle<StandardMaterial>>,
}

impl FluidMaterials {
    pub fn from_registry(
        fluids: &FluidRegistry,
        materials: &mut Assets<StandardMaterial>,
    ) -> Self {
        let materials = fluids
            .iter()
            .map(|(fluid_id, definition)| {
                let material = materials.add(StandardMaterial {
                    base_color: Color::srgba(
                        definition.color.r,
                        definition.color.g,
                        definition.color.b,
                        definition.opacity,
                    ),
                    perceptual_roughness: definition.roughness,
                    metallic: definition.metallic,
                    alpha_mode: AlphaMode::Blend,
                    double_sided: true,
                    cull_mode: None,
                    unlit: true,
                    ..default()
                });

                (fluid_id, material)
            })
            .collect();

        Self { materials }
    }

    fn get(&self, fluid_id: FluidId) -> &Handle<StandardMaterial> {
        self.materials
            .get(&fluid_id)
            .unwrap_or_else(|| panic!("missing material for fluid id {fluid_id}"))
    }
}

struct ChunkRenderSlot {
    entities: Vec<Entity>,
    meshes: Vec<Handle<Mesh>>,
}

#[derive(Resource, Default)]
pub struct ChunkRenderPool {
    active: HashMap<IVec3, ChunkRenderSlot>,
}

impl ChunkRenderPool {
    pub fn contains(&self, coord: IVec3) -> bool {
        self.active.contains_key(&coord)
    }

    pub fn active_coords(&self) -> impl Iterator<Item = IVec3> + '_ {
        self.active.keys().copied()
    }

    pub fn take(&mut self, coord: IVec3) -> Option<(Vec<Entity>, Vec<Handle<Mesh>>)> {
        self.active
            .remove(&coord)
            .map(|slot| (slot.entities, slot.meshes))
    }

    pub fn clear(&mut self, meshes: &mut Assets<Mesh>) {
        for (_, slot) in self.active.drain() {
            for handle in slot.meshes {
                let _ = meshes.remove(&handle);
            }
        }
    }

    fn insert(
        &mut self,
        coord: IVec3,
        entities: Vec<Entity>,
        meshes: Vec<Handle<Mesh>>,
    ) {
        self.active
            .insert(coord, ChunkRenderSlot { entities, meshes });
    }
}

pub fn spawn_chunk_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    world: &VoxelWorld,
    coord: IVec3,
    chunk: &VoxelChunk,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    terrain_materials: &TerrainMaterials,
    fluid_materials: &FluidMaterials,
) {
    if render_pool.contains(coord) {
        return;
    }

    if chunk.is_empty() {
        render_pool.insert(coord, Vec::new(), Vec::new());
        return;
    }

    let face_meshes = build_chunk_mesh(world, coord, chunk, |voxel, block_id| {
        if block_id != GRASS_BLOCK_ID {
            return [1.0, 1.0, 1.0];
        }

        let position = Vec2::new(voxel.x as f32 + 0.5, voxel.z as f32 + 0.5);
        let grass = biome_field.grass_color(position, biomes);
        [grass.r, grass.g, grass.b]
    });
    let fluid_meshes = build_fluid_meshes(world, coord, chunk);
    let chunk_size = CHUNK_SIZE as f32;
    let transform = Transform::from_translation(coord.as_vec3() * chunk_size);
    let mut entities = Vec::new();
    let mut mesh_handles = Vec::new();

    for face_mesh in face_meshes {
        let mesh_handle = meshes.add(face_mesh.mesh);
        let entity = commands
            .spawn((
                Mesh3d(mesh_handle.clone()),
                MeshMaterial3d(
                    terrain_materials
                        .for_face(face_mesh.block_id, face_mesh.face)
                        .clone(),
                ),
                transform,
                DespawnOnExit(GameState::Gameplay),
            ))
            .id();

        entities.push(entity);
        mesh_handles.push(mesh_handle);
    }

    for fluid_mesh in fluid_meshes {
        let mesh_handle = meshes.add(fluid_mesh.mesh);
        let entity = commands
            .spawn((
                Mesh3d(mesh_handle.clone()),
                MeshMaterial3d(fluid_materials.get(fluid_mesh.fluid_id).clone()),
                transform,
                NotShadowCaster,
                DespawnOnExit(GameState::Gameplay),
            ))
            .id();

        entities.push(entity);
        mesh_handles.push(mesh_handle);
    }

    render_pool.insert(coord, entities, mesh_handles);
}

pub fn refresh_adjacent_chunk_meshes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    world: &VoxelWorld,
    coord: IVec3,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    terrain_materials: &TerrainMaterials,
    fluid_materials: &FluidMaterials,
) {
    for offset in CHUNK_NEIGHBORS {
        let neighbor = coord + offset;

        if !render_pool.contains(neighbor) {
            continue;
        }

        refresh_chunk_mesh(
            commands,
            meshes,
            render_pool,
            world,
            neighbor,
            biomes,
            biome_field,
            terrain_materials,
            fluid_materials,
        );
    }
}

pub fn refresh_chunk_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    render_pool: &mut ChunkRenderPool,
    world: &VoxelWorld,
    coord: IVec3,
    biomes: &BiomeRegistry,
    biome_field: &BiomeField,
    terrain_materials: &TerrainMaterials,
    fluid_materials: &FluidMaterials,
) {
    if !render_pool.contains(coord) {
        return;
    }

    let Some(chunk) = world.chunk(coord) else {
        return;
    };

    if let Some((entities, mesh_handles)) = render_pool.take(coord) {
        for entity in entities {
            commands.entity(entity).despawn();
        }

        for handle in mesh_handles {
            let _ = meshes.remove(&handle);
        }
    }

    spawn_chunk_mesh(
        commands,
        meshes,
        render_pool,
        world,
        coord,
        chunk,
        biomes,
        biome_field,
        terrain_materials,
        fluid_materials,
    );
}

pub fn clear_chunk_render_pool(
    mut meshes: ResMut<Assets<Mesh>>,
    mut render_pool: ResMut<ChunkRenderPool>,
) {
    render_pool.clear(&mut meshes);
}
