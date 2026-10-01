use bevy::{
    asset::AssetId,
    ecs::system::SystemParam,
    gltf::GltfAssetLabel,
    light::{NotShadowCaster, NotShadowReceiver},
    platform::collections::HashMap,
    prelude::*,
};

use crate::{
    app::game_state::GameState,
    content::object::{ObjectDefinition, ObjectVisualDefinition},
    rendering::{
        block_tint::block_tint_at,
        color::{quantize_srgba, MATERIAL_TINT_RGB_LEVELS},
        extruded_sprite::{
            resolve_extruded_sprite_assets, ExtrudedSpriteAssetContext,
            ExtrudedSpriteAssetRequest, ExtrudedSpriteGeometry, ExtrudedSpriteMaterialCache,
            ExtrudedSpriteMeshCache,
        },
    },
    voxel::chunk::VoxelChunk,
};

use super::{
    world_object_transform, ObjectMaterialCache, ObjectMaterialKey, WorldObjectSceneContent,
};

const MAX_INSTANCES_PER_BATCH: usize = 256;

#[derive(SystemParam)]
pub(super) struct WorldObjectBatchAssets<'w> {
    images: Res<'w, Assets<Image>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    extruded_mesh_cache: ResMut<'w, ExtrudedSpriteMeshCache>,
    extruded_material_cache: ResMut<'w, ExtrudedSpriteMaterialCache>,
    object_material_cache: ResMut<'w, ObjectMaterialCache>,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct ObjectBatchKey {
    source_mesh: AssetId<Mesh>,
    material: AssetId<StandardMaterial>,
    casts_shadow: bool,
    receives_shadow: bool,
}

struct ObjectBatchAccumulator {
    mesh: Mesh,
    material: Handle<StandardMaterial>,
    instances: usize,
}

pub(super) struct BuiltWorldObjectChunk {
    pub(super) entities: Vec<Entity>,
    pub(super) object_count: usize,
    pub(super) batch_count: usize,
}

pub(super) fn build_world_object_chunk(
    commands: &mut Commands,
    coord: IVec3,
    chunk: &VoxelChunk,
    content: &WorldObjectSceneContent<'_>,
    assets: &mut WorldObjectBatchAssets<'_>,
) -> Option<BuiltWorldObjectChunk> {
    let chunk_origin = coord * crate::voxel::chunk::CHUNK_SIZE as i32;
    let chunk_origin_vec = chunk_origin.as_vec3();
    let mut batches: HashMap<ObjectBatchKey, Vec<ObjectBatchAccumulator>> = HashMap::new();
    let mut object_count = 0usize;

    for (x, y, z, object) in chunk.object_voxels() {
        let support = chunk_origin + IVec3::new(x as i32, y as i32, z as i32);
        let Some(definition) = content.objects.get(object.object_id) else {
            continue;
        };
        let support_cell = content.world.cell_at(support);
        let mut transform = world_object_transform(support, support_cell, object, definition);
        let tint = block_tint_at(
            definition.tint,
            Vec2::new(transform.translation.x, transform.translation.z),
            &content.biome_field,
            &content.biomes,
        );

        let (source_mesh, material) =
            resolve_object_render_assets(definition, tint, content, assets)?;
        let base_mesh = assets.meshes.get(&source_mesh).cloned()?;

        transform.translation -= chunk_origin_vec;
        let instance_mesh = base_mesh.transformed_by(transform);
        let key = ObjectBatchKey {
            source_mesh: source_mesh.id(),
            material: material.id(),
            casts_shadow: definition.casts_shadow,
            receives_shadow: definition.receives_shadow,
        };
        let segments = batches.entry(key).or_default();
        if segments
            .last()
            .is_none_or(|segment| segment.instances >= MAX_INSTANCES_PER_BATCH)
        {
            segments.push(ObjectBatchAccumulator {
                mesh: instance_mesh,
                material: material.clone(),
                instances: 1,
            });
        } else {
            let segment = segments
                .last_mut()
                .expect("non-empty object batch segment list must have a last element");
            if let Err(error) = segment.mesh.merge(&instance_mesh) {
                warn!(
                    "cannot merge world-object mesh for {} in chunk {coord:?}: {error}",
                    definition.id
                );
                segments.push(ObjectBatchAccumulator {
                    mesh: instance_mesh,
                    material: material.clone(),
                    instances: 1,
                });
            } else {
                segment.instances += 1;
            }
        }
        object_count += 1;
    }

    let batch_count = batches.values().map(Vec::len).sum();
    let mut entities = Vec::with_capacity(batch_count);
    for (key, segments) in batches {
        for segment in segments {
            let mesh = assets.meshes.add(segment.mesh);
            let mut entity = commands.spawn((
                Name::new(format!(
                    "World Object Batch ({coord:?}, {} instances)",
                    segment.instances
                )),
                Mesh3d(mesh),
                MeshMaterial3d(segment.material),
                Transform::from_translation(chunk_origin_vec),
                Visibility::Visible,
                DespawnOnExit(GameState::Gameplay),
            ));
            if !key.casts_shadow {
                entity.insert(NotShadowCaster);
            }
            if !key.receives_shadow {
                entity.insert(NotShadowReceiver);
            }
            entities.push(entity.id());
        }
    }

    Some(BuiltWorldObjectChunk {
        entities,
        object_count,
        batch_count,
    })
}

fn resolve_object_render_assets(
    definition: &ObjectDefinition,
    tint: Color,
    content: &WorldObjectSceneContent<'_>,
    assets: &mut WorldObjectBatchAssets<'_>,
) -> Option<(Handle<Mesh>, Handle<StandardMaterial>)> {
    match &definition.visual {
        ObjectVisualDefinition::Model { path } => {
            let mesh: Handle<Mesh> = content.asset_server.load(
                GltfAssetLabel::Primitive {
                    mesh: 0,
                    primitive: 0,
                }
                .from_asset(path.clone()),
            );
            assets.meshes.get(&mesh)?;

            let source_material: Handle<StandardMaterial> =
                content.asset_server.load(format!("{path}#Material0/std"));
            let material = resolve_model_material(
                source_material,
                tint,
                definition.unlit,
                &mut assets.materials,
                &mut assets.object_material_cache,
            )?;
            Some((mesh, material))
        }
        ObjectVisualDefinition::ExtrudedSprite {
            texture,
            base_offset,
            height,
            size,
            alpha_cutoff,
        } => {
            let request = ExtrudedSpriteAssetRequest {
                texture: content.asset_server.load(texture.clone()),
                geometry: ExtrudedSpriteGeometry {
                    size: *size,
                    height: *height,
                    base_offset: *base_offset,
                    alpha_cutoff: *alpha_cutoff,
                },
                tint,
                unlit: definition.unlit,
            };
            let context = ExtrudedSpriteAssetContext {
                images: &assets.images,
                meshes: &mut assets.meshes,
                materials: &mut assets.materials,
                mesh_cache: &mut assets.extruded_mesh_cache,
                material_cache: &mut assets.extruded_material_cache,
            };
            match resolve_extruded_sprite_assets(request, context) {
                Ok(Some(resolved)) => Some(resolved),
                Ok(None) => None,
                Err(error) => {
                    warn!(
                        "cannot build world-object extruded sprite {}: {error}",
                        definition.id
                    );
                    None
                }
            }
        }
    }
}

fn resolve_model_material(
    source: Handle<StandardMaterial>,
    tint: Color,
    unlit: bool,
    materials: &mut Assets<StandardMaterial>,
    cache: &mut ObjectMaterialCache,
) -> Option<Handle<StandardMaterial>> {
    let (tint, tint_key) = quantize_srgba(tint, MATERIAL_TINT_RGB_LEVELS);
    let key = ObjectMaterialKey {
        material: source.id(),
        tint: tint_key,
        unlit,
    };
    if let Some(existing) = cache.0.get(&key) {
        return Some(existing.clone());
    }

    let mut material = materials.get(&source).cloned()?;
    material.base_color = tint;
    material.unlit = unlit;
    let handle = materials.add(material);
    cache.0.insert(key, handle.clone());
    Some(handle)
}
