use bevy::{
    asset::AssetId,
    ecs::system::SystemParam,
    gltf::GltfAssetLabel,
    platform::collections::{HashMap, HashSet},
    prelude::*,
    world_serialization::WorldInstanceReady,
};

use crate::{
    app::game_state::GameState,
    content::{
        biome::BiomeRegistry,
        block::{BlockDefinition, BlockRegistry, BlockTint},
        builtin_ids::BIOME_TINT_METADATA_KEY,
    },
    rendering::block_tint::{block_tint_at, block_tint_for_biome},
    voxel::{cell::VoxelCell, chunk::CHUNK_SIZE, world::VoxelWorld},
    world::biome_field::BiomeField,
};

use super::{ChunkRenderCoord, ChunkRenderPool};

#[derive(Resource, Default)]
pub(crate) struct CustomBlockModelRenderPool {
    chunks: HashMap<IVec3, CustomBlockModelChunk>,
}

struct CustomBlockModelChunk {
    revision: u64,
    entities: Vec<Entity>,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct CustomBlockMaterialKey {
    material: AssetId<StandardMaterial>,
    tint: [u32; 4],
}

#[derive(Resource, Default)]
pub(crate) struct CustomBlockModelMaterials(
    HashMap<CustomBlockMaterialKey, Handle<StandardMaterial>>,
);

#[derive(Component)]
struct CustomBlockModelTint(Color);

pub(crate) fn clear_custom_block_model_render_pool(
    mut commands: Commands,
    mut pool: ResMut<CustomBlockModelRenderPool>,
) {
    for chunk in pool.chunks.drain().map(|(_, chunk)| chunk) {
        despawn_model_entities(&mut commands, chunk.entities);
    }
}

#[derive(SystemParam)]
pub(crate) struct CustomBlockModelContent<'w> {
    world: Res<'w, VoxelWorld>,
    blocks: Res<'w, BlockRegistry>,
    biomes: Res<'w, BiomeRegistry>,
    biome_field: Res<'w, BiomeField>,
    asset_server: Res<'w, AssetServer>,
    render_pool: Res<'w, ChunkRenderPool>,
}

pub(crate) fn sync_custom_block_models(
    mut commands: Commands,
    content: CustomBlockModelContent,
    mut model_pool: ResMut<CustomBlockModelRenderPool>,
) {
    let active = content
        .render_pool
        .active_coords()
        .collect::<HashSet<_>>();

    let retired = model_pool
        .chunks
        .keys()
        .copied()
        .filter(|coord| !active.contains(coord))
        .collect::<Vec<_>>();
    for coord in retired {
        if let Some(chunk) = model_pool.chunks.remove(&coord) {
            despawn_model_entities(&mut commands, chunk.entities);
        }
    }

    for coord in active {
        let Some(revision) = content.world.chunk_content_revision(coord) else {
            continue;
        };
        if model_pool
            .chunks
            .get(&coord)
            .is_some_and(|chunk| chunk.revision == revision)
        {
            continue;
        }

        if let Some(previous) = model_pool.chunks.remove(&coord) {
            despawn_model_entities(&mut commands, previous.entities);
        }

        let Some(chunk) = content.world.chunk(coord) else {
            continue;
        };
        let mut entities = Vec::new();
        let chunk_origin = coord * CHUNK_SIZE as i32;
        chunk.visit_block_voxels(|x, y, z, cell| {
            let Some(block) = content.blocks.get(cell.block_id) else {
                return;
            };
            let Some(model) = block.model.as_ref() else {
                return;
            };

            let world_voxel =
                chunk_origin + IVec3::new(x as i32, y as i32, z as i32);
            let tint = custom_model_tint(
                world_voxel,
                cell,
                block,
                &content.biome_field,
                &content.biomes,
            );
            let scene = content.asset_server.load(
                GltfAssetLabel::Scene(0).from_asset(model.clone()),
            );
            let entity = commands
                .spawn((
                    Name::new(format!("Block Model ({})", block.id)),
                    WorldAssetRoot(scene),
                    Transform::from_translation(
                        world_voxel.as_vec3() + Vec3::new(0.5, 0.0, 0.5),
                    ),
                    Visibility::Hidden,
                    ChunkRenderCoord(coord),
                    CustomBlockModelTint(tint),
                    DespawnOnExit(GameState::Gameplay),
                ))
                .observe(apply_custom_block_model_tint)
                .id();
            entities.push(entity);
        });

        model_pool
            .chunks
            .insert(coord, CustomBlockModelChunk { revision, entities });
    }
}

fn custom_model_tint(
    voxel: IVec3,
    cell: &VoxelCell,
    block: &BlockDefinition,
    biome_field: &BiomeField,
    biomes: &BiomeRegistry,
) -> Color {
    if block.tint == BlockTint::None {
        return Color::WHITE;
    }

    if let Some(tint) = cell
        .secondary_property(BIOME_TINT_METADATA_KEY)
        .and_then(|biome_id| block_tint_for_biome(block.tint, biome_id, biomes))
    {
        return tint;
    }

    block_tint_at(
        block.tint,
        Vec2::new(voxel.x as f32 + 0.5, voxel.z as f32 + 0.5),
        biome_field,
        biomes,
    )
}

fn apply_custom_block_model_tint(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    descendants: Query<&Children>,
    tints: Query<&CustomBlockModelTint>,
    mesh_materials: Query<&MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: ResMut<CustomBlockModelMaterials>,
) {
    let Ok(tint) = tints.get(ready.entity) else {
        return;
    };
    let rgba = tint.0.to_srgba();
    let tint_bits = [
        rgba.red.to_bits(),
        rgba.green.to_bits(),
        rgba.blue.to_bits(),
        rgba.alpha.to_bits(),
    ];

    for descendant in descendants.iter_descendants(ready.entity) {
        let Ok(original) = mesh_materials.get(descendant) else {
            continue;
        };
        let key = CustomBlockMaterialKey {
            material: original.id(),
            tint: tint_bits,
        };
        let replacement = if let Some(existing) = cache.0.get(&key) {
            existing.clone()
        } else {
            let Some(mut material) = materials.get(original.id()).cloned() else {
                continue;
            };
            material.base_color = tint.0;
            let handle = materials.add(material);
            cache.0.insert(key, handle.clone());
            handle
        };
        commands
            .entity(descendant)
            .insert(MeshMaterial3d(replacement));
    }
}

fn despawn_model_entities(commands: &mut Commands, entities: Vec<Entity>) {
    for entity in entities {
        commands.entity(entity).despawn();
    }
}
