use bevy::{
    asset::AssetId,
    camera::visibility::RenderLayers,
    ecs::system::SystemParam,
    gltf::GltfMaterialName,
    prelude::*,
    world_serialization::WorldInstanceReady,
};

use crate::{
    content::creature::CreatureRegistry,
    creatures::CreatureInstance,
    localization::ActiveLanguage,
    targeting::block::TargetedCreature,
};

use super::super::{HudSettings, TargetBlockPosition};

const PORTRAIT_RENDER_LAYER: usize = 2;

#[derive(Component)]
pub(super) struct PortraitCamera;

#[derive(Component)]
struct PortraitModel {
    definition_id: String,
    gltf: Handle<Gltf>,
    scene_attached: bool,
}

#[derive(Component)]
struct PortraitAppearance(String);

#[derive(SystemParam)]
struct PortraitSelection<'w, 's> {
    target: Res<'w, TargetedCreature>,
    settings: Res<'w, HudSettings>,
    creatures: Query<'w, 's, &'static CreatureInstance>,
    definitions: Res<'w, CreatureRegistry>,
    asset_server: Res<'w, AssetServer>,
}

/// Only one portrait scene exists, even if the target changes every frame.
/// Its camera and meshes are confined to a layer invisible to world cameras.
pub(super) fn sync_portrait(
    mut commands: Commands,
    selection: PortraitSelection,
    mut camera: Single<(&mut Camera, &mut Transform), With<PortraitCamera>>,
    portraits: Query<(Entity, &PortraitModel)>,
) {
    let target = if selection.settings.target_block_position() == TargetBlockPosition::Hidden {
        None
    } else {
        selection.target.0
            .and_then(|entity| selection.creatures.get(entity).ok())
            .and_then(|creature| selection.definitions.get(&creature.definition_id))
    };
    let (camera_settings, camera_transform) = &mut *camera;
    camera_settings.is_active = target.is_some();
    let Some(definition) = target else {
        for (entity, _) in &portraits {
            commands.entity(entity).despawn();
        }
        return;
    };

    let size = Vec3::from_array(definition.collider.size).max_element();
    let distance = size * 2.4 + 0.45;
    **camera_transform = Transform::from_xyz(distance * 0.4, distance * 0.25, distance)
        .looking_at(Vec3::ZERO, Vec3::Y);

    if portraits.iter().any(|(_, model)| model.definition_id == definition.id) {
        return;
    }
    for (entity, _) in &portraits {
        commands.entity(entity).despawn();
    }
    commands.spawn((
        PortraitModel {
            definition_id: definition.id.clone(),
            gltf: selection.asset_server.load(definition.model.clone()),
            scene_attached: false,
        },
        Transform::default(),
        RenderLayers::layer(PORTRAIT_RENDER_LAYER),
    ));
}

/// Attach the glTF only after it is available. Keep its native geometry,
/// rather than approximating every species with the same square icon.
pub(super) fn prepare_portrait(
    mut commands: Commands,
    mut portraits: Query<(Entity, &mut PortraitModel)>,
    gltfs: Res<Assets<Gltf>>,
    definitions: Res<CreatureRegistry>,
) {
    for (entity, mut portrait) in &mut portraits {
        if portrait.scene_attached {
            continue;
        }
        let Some(gltf) = gltfs.get(&portrait.gltf) else {
            continue;
        };
        let Some(scene) = gltf.default_scene.clone() else {
            warn!("portrait for {} has no default glTF scene", portrait.definition_id);
            portrait.scene_attached = true;
            continue;
        };
        let Some(definition) = definitions.get(&portrait.definition_id) else {
            continue;
        };
        let appearance = PortraitAppearance(portrait.definition_id.clone());
        let center_y = definition.collider.center_offset[1];
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                WorldAssetRoot(scene),
                Transform::from_xyz(0.0, -center_y, 0.0),
                appearance,
            ))
            .observe(configure_portrait_scene);
        });
        portrait.scene_attached = true;
    }
}

#[derive(SystemParam)]
struct PortraitMaterials<'w> {
    definitions: Res<'w, CreatureRegistry>,
    asset_server: Res<'w, AssetServer>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

/// Apply exactly the species material tints and texture overrides that are
/// applied to the gameplay creature, leaving the original glTF assets intact.
fn configure_portrait_scene(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    descendants: Query<&Children>,
    appearances: Query<&PortraitAppearance>,
    meshes: Query<(), With<Mesh3d>>,
    named_materials: Query<(&MeshMaterial3d<StandardMaterial>, &GltfMaterialName)>,
    mut assets: PortraitMaterials,
) {
    let Ok(appearance) = appearances.get(ready.entity) else {
        return;
    };
    let Some(definition) = assets.definitions.get(&appearance.0) else {
        return;
    };
    for entity in descendants.iter_descendants(ready.entity) {
        if meshes.contains(entity) {
            commands.entity(entity).insert(RenderLayers::layer(PORTRAIT_RENDER_LAYER));
        }
        let Ok((original, material_name)) = named_materials.get(entity) else {
            continue;
        };
        let name = material_name.0.as_str();
        let tint = definition.material_tints.get(name);
        let texture = definition.textures.get(name);
        if tint.is_none() && texture.is_none() {
            continue;
        }
        let Some(mut material) = assets.materials.get(original.id()).cloned() else {
            continue;
        };
        if let Some(tint) = tint {
            let rgb = tint.to_srgb();
            let alpha = material.base_color.to_srgba().alpha;
            material.base_color = Color::srgba(rgb[0], rgb[1], rgb[2], alpha);
        }
        if let Some(path) = texture {
            material.base_color_texture = Some(assets.asset_server.load(path.clone()));
        }
        let handle = assets.materials.add(material);
        commands.entity(entity).insert(MeshMaterial3d(handle));
    }
}
