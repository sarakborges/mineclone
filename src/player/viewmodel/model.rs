use std::{collections::HashSet, f32::consts::PI};

use bevy::{
    camera::{Hdr, visibility::RenderLayers},
    ecs::system::SystemParam,
    light::NotShadowCaster,
    prelude::*,
    world_serialization::WorldInstanceReady,
};

use crate::{
    content::{block::BlockDefinition, player::PlayerDefinition},
    player::{
        apply_player_skin_material,
        camera::GameplayCamera,
        hotbar::PlayerHotbar,
    },
    rendering::{
        block_model::{
            BlockModel, BlockModelMaterials, BlockModelMeshes, apply_block_display_shading,
            block_face_material_data, maximum_block_model_layers, set_block_model_tint,
        },
        block_model_material::BlockModelMaterial,
        block_visual_content::BlockVisualContent,
        camera_stack::VIEW_MODEL_CAMERA_ORDER,
    },
    voxel::block_face::BlockFace,
};

use super::animation::{PlayerViewModel, ViewModelItemSwitch, base_viewmodel_transform};

const MODEL_RIGHT_ARM_PIVOT: Vec3 = Vec3::new(-0.3375, 1.35, 0.0);
pub(super) const VIEW_MODEL_ARM_LENGTH: f32 = 0.675;
pub(super) const VIEW_MODEL_ARM_GRIP_Y: f32 = 0.52;
const HELD_BLOCK_SCALE: f32 = 0.18;
const VIEW_MODEL_RENDER_LAYER: usize = 1;

#[derive(Component)]
pub(super) struct ViewModelArm;

#[derive(Component)]
pub(super) struct ViewModelArmScene(Handle<Gltf>);

#[derive(Component)]
pub(super) struct ViewModelArmSceneAttached;

#[derive(Component)]
struct ViewModelArmAppearance {
    owner: Entity,
}

#[derive(Component)]
pub(super) struct HeldBlockRoot;

#[derive(Component)]
pub(super) struct HeldBlockFace {
    face: BlockFace,
    layer_index: usize,
}

#[derive(Default)]
pub(super) struct HeldBlockVisualCache {
    tint_cell: Option<IVec2>,
    tint: Option<Color>,
}

type HeldBlockRootQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut BlockModel, &'static mut Visibility),
    (
        With<HeldBlockRoot>,
        Without<HeldBlockFace>,
        Without<ViewModelArm>,
        Without<PlayerViewModel>,
        Without<GameplayCamera>,
    ),
>;

#[derive(SystemParam)]
pub(super) struct ViewModelSelection<'w> {
    hotbar: Res<'w, PlayerHotbar>,
}

#[derive(SystemParam)]
pub(super) struct ViewModelSpawnAssets<'w> {
    player_definition: Res<'w, PlayerDefinition>,
    asset_server: Res<'w, AssetServer>,
    block_meshes: Res<'w, BlockModelMeshes>,
    block_materials: ResMut<'w, BlockModelMaterials>,
    materials: ResMut<'w, Assets<BlockModelMaterial>>,
}

#[derive(SystemParam)]
struct ViewModelArmSceneAssets<'w, 's> {
    asset_server: Res<'w, AssetServer>,
    names: Query<'w, 's, &'static Name>,
    meshes: Query<'w, 's, &'static Mesh3d>,
    mesh_materials: Query<'w, 's, &'static MeshMaterial3d<StandardMaterial>>,
    appearances: Query<'w, 's, &'static ViewModelArmAppearance>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

#[derive(SystemParam)]
pub(super) struct HeldBlockView<'w, 's> {
    materials: ResMut<'w, Assets<BlockModelMaterial>>,
    roots: HeldBlockRootQuery<'w, 's>,
    faces: Query<
        'w,
        's,
        (
            &'static HeldBlockFace,
            &'static MeshMaterial3d<BlockModelMaterial>,
            &'static mut Visibility,
        ),
        Without<HeldBlockRoot>,
    >,
}

pub(super) fn spawn_viewmodel(
    mut commands: Commands,
    cameras: Query<(Entity, &Transform), Added<GameplayCamera>>,
    definitions: BlockVisualContent,
    selection: ViewModelSelection,
    assets: ViewModelSpawnAssets,
    mut item_switch: ResMut<ViewModelItemSwitch>,
) {
    let ViewModelSpawnAssets {
        player_definition,
        asset_server,
        block_meshes,
        mut block_materials,
        mut materials,
    } = assets;
    let player_model = player_definition
        .model
        .as_ref()
        .map(|path| asset_server.load::<Gltf>(path.clone()));
    if player_model.is_none() {
        warn!("player definition has no model configured for the first-person arm");
    }

    for (camera, camera_transform) in &cameras {
        let selected_slot = selection.hotbar.selected_slot();
        let selected_block_id = selection
            .hotbar
            .item_at(selected_slot)
            .filter(|block_id| definitions.blocks.get(block_id).is_some());
        item_switch.initialize(selected_block_id);

        let item_visibility = item_visibility(selected_block_id);
        let tint_position = Vec2::new(
            camera_transform.translation.x,
            camera_transform.translation.z,
        );
        let block_model = selected_block_id
            .map(BlockModel::display)
            .unwrap_or_else(BlockModel::empty_display);

        commands.entity(camera).with_children(|camera| {
            camera.spawn((
                Camera3d::default(),
                Camera {
                    order: VIEW_MODEL_CAMERA_ORDER,
                    clear_color: ClearColorConfig::None,
                    ..default()
                },
                Hdr,
                Msaa::Off,
                RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
            ));

            camera
                .spawn((
                    PlayerViewModel,
                    base_viewmodel_transform(),
                    Visibility::Inherited,
                ))
                .with_children(|viewmodel| {
                    if let Some(model) = player_model.clone() {
                        viewmodel.spawn((
                            Name::new("First Person Player Arm"),
                            ViewModelArmScene(model),
                            viewmodel_arm_scene_transform(),
                            Visibility::Hidden,
                        ));
                    }

                    viewmodel
                        .spawn((
                            HeldBlockRoot,
                            block_model,
                            held_block_transform(),
                            item_visibility,
                        ))
                        .with_children(|held| {
                            let selected_block = selected_block_id.and_then(|block_id| {
                                definitions
                                    .blocks
                                    .get(block_id)
                                    .map(|block| (block_id, block))
                            });
                            let tint = selected_block.and_then(|(block_id, _)| {
                                definitions.tint_at(block_id, tint_position)
                            });

                            for &face in block_model.faces() {
                                let layer_count =
                                    maximum_block_model_layers(&definitions.blocks, face);
                                let layer_materials = block_materials.held_for_face(
                                    face,
                                    layer_count,
                                    &mut materials,
                                );

                                for (layer_index, material) in
                                    layer_materials.into_iter().enumerate()
                                {
                                    let mut visibility = Visibility::Hidden;

                                    if let Some((_, block)) = selected_block
                                        && let Some(face_material) = held_block_face_material(
                                            block,
                                            &definitions.asset_server,
                                            face,
                                            layer_index,
                                            block_model.opacity(),
                                        )
                                        && let Some(mut material_asset) =
                                            materials.get_mut(&material)
                                    {
                                        *material_asset = face_material;
                                        set_block_model_tint(
                                            &mut material_asset,
                                            tint.unwrap_or(Color::WHITE),
                                        );
                                        visibility = Visibility::Inherited;
                                    }

                                    held.spawn((
                                        HeldBlockFace { face, layer_index },
                                        Mesh3d(block_meshes.display_face(face)),
                                        MeshMaterial3d(material),
                                        visibility,
                                        RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
                                        NotShadowCaster,
                                    ));
                                }
                            }
                        });
                });
        });
    }
}

pub(super) fn attach_viewmodel_arm_model(
    mut commands: Commands,
    sources: Query<(Entity, &ViewModelArmScene), Without<ViewModelArmSceneAttached>>,
    gltfs: Res<Assets<Gltf>>,
) {
    for (source, model) in &sources {
        let Some(gltf) = gltfs.get(&model.0) else {
            continue;
        };
        let Some(scene) = gltf.default_scene.clone() else {
            warn!("player model has no default scene for the first-person arm");
            commands.entity(source).insert(ViewModelArmSceneAttached);
            continue;
        };

        commands.entity(source).insert(ViewModelArmSceneAttached);
        commands.entity(source).with_children(|parent| {
            parent
                .spawn((
                    WorldAssetRoot(scene),
                    Transform::default(),
                    ViewModelArmAppearance { owner: source },
                ))
                .observe(configure_viewmodel_arm_scene);
        });
    }
}

fn configure_viewmodel_arm_scene(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    descendants: Query<&Children>,
    mut assets: ViewModelArmSceneAssets,
) {
    let Ok(appearance) = assets.appearances.get(ready.entity) else {
        return;
    };

    let Some(arm_root) = descendants
        .iter_descendants(ready.entity)
        .find(|entity| {
            assets
                .names
                .get(*entity)
                .is_ok_and(|name| name.as_str() == "RightArmPivot")
        })
    else {
        warn!("player model is missing RightArmPivot for the first-person arm");
        return;
    };

    let mut arm_entities = HashSet::from([arm_root]);
    arm_entities.extend(descendants.iter_descendants(arm_root));

    for descendant in descendants.iter_descendants(ready.entity) {
        if assets.meshes.get(descendant).is_err() {
            continue;
        }

        if !arm_entities.contains(&descendant) {
            commands.entity(descendant).insert(Visibility::Hidden);
            continue;
        }

        commands.entity(descendant).insert((
            ViewModelArm,
            Visibility::Inherited,
            RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
            NotShadowCaster,
        ));

        if let Ok(original) = assets.mesh_materials.get(descendant)
            && let Some(mut material) = assets.materials.get(original.id()).cloned()
        {
            apply_player_skin_material(&mut material, &assets.asset_server);
            let material = assets.materials.add(material);
            commands
                .entity(descendant)
                .insert(MeshMaterial3d(material));
        }
    }

    commands
        .entity(appearance.owner)
        .insert(Visibility::Inherited);
}

pub(super) fn sync_held_block(
    definitions: BlockVisualContent,
    selection: ViewModelSelection,
    player: Single<&Transform, With<GameplayCamera>>,
    mut cache: Local<HeldBlockVisualCache>,
    view: HeldBlockView,
) {
    let HeldBlockView {
        mut materials,
        mut roots,
        mut faces,
    } = view;
    let tint_cell = IVec2::new(
        player.translation.x.floor() as i32,
        player.translation.z.floor() as i32,
    );
    let tint_cell_changed = cache.tint_cell != Some(tint_cell);
    let block_definitions_changed = definitions.block_definitions_changed();
    let selection_changed = selection.hotbar.is_changed();
    let visual_inputs_changed = definitions.inputs_changed();
    if !tint_cell_changed && !selection_changed && !visual_inputs_changed {
        return;
    }
    cache.tint_cell = Some(tint_cell);

    let selected_slot = selection.hotbar.selected_slot();
    let selected_block_id = selection
        .hotbar
        .item_at(selected_slot)
        .filter(|block_id| definitions.blocks.get(block_id).is_some());
    let visibility = item_visibility(selected_block_id);
    let tint_position = tint_cell.as_vec2() + Vec2::splat(0.5);

    for (mut held, mut held_visibility) in &mut roots {
        let block_changed = held.block_id() != selected_block_id;
        if block_changed {
            held.set_block_id(selected_block_id);
        }

        if *held_visibility != visibility {
            *held_visibility = visibility;
        }

        let Some(block_id) = selected_block_id else {
            if block_changed {
                for (_, _, mut layer_visibility) in &mut faces {
                    if *layer_visibility != Visibility::Hidden {
                        *layer_visibility = Visibility::Hidden;
                    }
                }
            }
            cache.tint = None;
            continue;
        };
        let block = definitions
            .blocks
            .get(block_id)
            .unwrap_or_else(|| panic!("hotbar references missing block: {block_id}"));

        let materials_changed = block_changed || block_definitions_changed;
        if materials_changed {
            for (face, material_handle, mut layer_visibility) in &mut faces {
                let Some(mut material) = materials.get_mut(&material_handle.0) else {
                    continue;
                };

                let Some(face_material) = held_block_face_material(
                    block,
                    &definitions.asset_server,
                    face.face,
                    face.layer_index,
                    held.opacity(),
                ) else {
                    if *layer_visibility != Visibility::Hidden {
                        *layer_visibility = Visibility::Hidden;
                    }
                    continue;
                };

                *material = face_material;
                if *layer_visibility != Visibility::Inherited {
                    *layer_visibility = Visibility::Inherited;
                }
            }
        }

        if materials_changed || tint_cell_changed || visual_inputs_changed {
            let tint = definitions
                .tint_at(block_id, tint_position)
                .unwrap_or(Color::WHITE);
            if materials_changed || cache.tint != Some(tint) {
                for (_, material_handle, _) in &mut faces {
                    let Some(mut material) = materials.get_mut(&material_handle.0) else {
                        continue;
                    };

                    set_block_model_tint(&mut material, tint);
                }
            }
            cache.tint = Some(tint);
        }
    }
}

fn held_block_face_material(
    block: &BlockDefinition,
    asset_server: &AssetServer,
    face: BlockFace,
    layer_index: usize,
    opacity: f32,
) -> Option<BlockModelMaterial> {
    let mut material =
        block_face_material_data(face, layer_index, block, asset_server, opacity)?;
    apply_block_display_shading(&mut material, face, opacity);
    Some(material)
}

fn item_visibility(block_id: Option<&'static str>) -> Visibility {
    if block_id.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

fn held_block_transform() -> Transform {
    // Presentation is independent of the placement orientation selected by R.
    // Keep the held cube upright in camera space without mutating its geometry.
    let viewmodel_rotation = base_viewmodel_transform().rotation;
    Transform::from_translation(Vec3::new(-0.02, VIEW_MODEL_ARM_LENGTH + 0.04, 0.20))
        .with_rotation(viewmodel_rotation.inverse())
        .with_scale(Vec3::splat(HELD_BLOCK_SCALE))
}

fn viewmodel_arm_scene_transform() -> Transform {
    let rotation = Quat::from_rotation_z(PI);
    Transform::from_translation(-(rotation * MODEL_RIGHT_ARM_PIVOT)).with_rotation(rotation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn held_block_geometry_has_fixed_display_scale_and_rotation() {
        let first = held_block_transform();
        let second = held_block_transform();

        assert_eq!(first.rotation, second.rotation);
        assert_eq!(first.scale, Vec3::splat(HELD_BLOCK_SCALE));
        assert_eq!(first.translation, second.translation);
    }
}
