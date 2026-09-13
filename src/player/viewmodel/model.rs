use bevy::{
    camera::visibility::RenderLayers, ecs::system::SystemParam, light::NotShadowCaster, prelude::*,
    ui::IsDefaultUiCamera,
};

use crate::{
    content::{
        biome::BiomeRegistry, block::BlockRegistry, block_orientation::BlockOrientation,
    },
    player::{camera::GameplayCamera, hotbar::PlayerHotbar},
    rendering::{
        block_model::{
            BlockModel, BlockModelMaterials, BlockModelMeshes, apply_block_display_shading,
            block_face_material_data, maximum_block_model_layers, set_block_model_tint,
        },
        block_model_material::BlockModelMaterial,
        block_tint::block_tint_at,
    },
    targeting::PlacementOrientation,
    voxel::{block_face::BlockFace, orientation::orientation_rotation},
    world::biome_field::BiomeField,
};

use super::animation::{PlayerViewModel, ViewModelItemSwitch, base_viewmodel_transform};

const ARM_SIZE: Vec3 = Vec3::new(0.23, 0.60, 0.21);
const HELD_BLOCK_SCALE: f32 = 0.18;
const VIEW_MODEL_RENDER_LAYER: usize = 1;

#[derive(Component)]
pub(super) struct ViewModelArm;

#[derive(Component)]
pub(super) struct HeldBlockRoot;

#[derive(Component)]
pub(super) struct HeldBlockFace {
    face: BlockFace,
    layer_index: usize,
}

pub(super) type HeldBlockRootQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut BlockModel,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (
        With<HeldBlockRoot>,
        Without<HeldBlockFace>,
        Without<ViewModelArm>,
        Without<PlayerViewModel>,
        Without<GameplayCamera>,
    ),
>;

#[derive(Resource)]
pub(super) struct ViewModelArmAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

#[derive(SystemParam)]
pub(super) struct ViewModelContent<'w> {
    asset_server: Res<'w, AssetServer>,
    blocks: Res<'w, BlockRegistry>,
    biomes: Res<'w, BiomeRegistry>,
    biome_field: Res<'w, BiomeField>,
    hotbar: Res<'w, PlayerHotbar>,
    placement_orientation: Res<'w, PlacementOrientation>,
}

pub(super) fn setup_viewmodel_arm_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(ViewModelArmAssets {
        mesh: meshes.add(Cuboid::new(ARM_SIZE.x, ARM_SIZE.y, ARM_SIZE.z)),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.72, 0.52, 0.40),
            perceptual_roughness: 1.0,
            unlit: true,
            ..default()
        }),
    });
}

pub(super) fn spawn_viewmodel(
    mut commands: Commands,
    cameras: Query<(Entity, &Transform), Added<GameplayCamera>>,
    content: ViewModelContent,
    block_meshes: Res<BlockModelMeshes>,
    mut block_materials: ResMut<BlockModelMaterials>,
    arm_assets: Res<ViewModelArmAssets>,
    mut item_switch: ResMut<ViewModelItemSwitch>,
    mut materials: ResMut<Assets<BlockModelMaterial>>,
) {
    for (camera, camera_transform) in &cameras {
        let selected_slot = content.hotbar.selected_slot();
        let selected_block_id = content
            .hotbar
            .item_at(selected_slot)
            .filter(|block_id| content.blocks.get(block_id).is_some());
        item_switch.initialize(selected_block_id);

        let item_visibility = item_visibility(selected_block_id);
        let tint_position = Vec2::new(
            camera_transform.translation.x,
            camera_transform.translation.z,
        );
        let block_model = selected_block_id
            .map(BlockModel::display)
            .unwrap_or_else(BlockModel::empty_display);
        let held_orientation = selected_block_id
            .and_then(|block_id| content.blocks.get(block_id))
            .map_or(BlockOrientation::default(), |block| {
                content
                    .placement_orientation
                    .for_block(selected_slot, block)
            });

        commands.entity(camera).with_children(|camera| {
            camera.spawn((
                Camera3d::default(),
                Camera {
                    order: 1,
                    clear_color: ClearColorConfig::None,
                    ..default()
                },
                Msaa::Off,
                RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
                IsDefaultUiCamera,
            ));

            camera
                .spawn((
                    PlayerViewModel,
                    base_viewmodel_transform(),
                    Visibility::Visible,
                ))
                .with_children(|viewmodel| {
                    viewmodel.spawn((
                        ViewModelArm,
                        Mesh3d(arm_assets.mesh.clone()),
                        MeshMaterial3d(arm_assets.material.clone()),
                        Transform::from_translation(Vec3::new(0.0, ARM_SIZE.y * 0.5, 0.0)),
                        Visibility::Visible,
                        RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
                        NotShadowCaster,
                    ));

                    viewmodel
                        .spawn((
                            HeldBlockRoot,
                            block_model,
                            held_block_transform(held_orientation),
                            item_visibility,
                        ))
                        .with_children(|held| {
                            let selected_block = selected_block_id.and_then(|block_id| {
                                content.blocks.get(block_id).map(|block| (block_id, block))
                            });
                            let tint = selected_block.map(|(_, block)| {
                                block_tint_at(
                                    block.tint,
                                    tint_position,
                                    &content.biome_field,
                                    &content.biomes,
                                )
                            });

                            for &face in block_model.faces() {
                                let layer_count = maximum_block_model_layers(&content.blocks, face);
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
                                        && let Some(face_material) = block_face_material_data(
                                            face,
                                            layer_index,
                                            block,
                                            &content.asset_server,
                                            block_model.opacity(),
                                        )
                                        && let Some(mut material_asset) = materials.get_mut(&material)
                                    {
                                        *material_asset = face_material;
                                        apply_block_display_shading(
                                            &mut material_asset,
                                            face,
                                            block_model.opacity(),
                                        );
                                        set_block_model_tint(
                                            &mut material_asset,
                                            tint.unwrap_or(Color::WHITE),
                                        );
                                        visibility = Visibility::Visible;
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

pub(super) fn sync_held_block(
    content: ViewModelContent,
    item_switch: Res<ViewModelItemSwitch>,
    player: Single<&Transform, With<GameplayCamera>>,
    mut materials: ResMut<Assets<BlockModelMaterial>>,
    mut roots: HeldBlockRootQuery,
    mut faces: Query<
        (
            &HeldBlockFace,
            &MeshMaterial3d<BlockModelMaterial>,
            &mut Visibility,
        ),
        Without<HeldBlockRoot>,
    >,
) {
    let selected_block_id = item_switch.displayed_block_id();
    let selected_slot = content.hotbar.selected_slot();
    let hotbar_block_id = content
        .hotbar
        .item_at(selected_slot)
        .filter(|block_id| content.blocks.get(block_id).is_some());
    let visibility = item_visibility(selected_block_id);
    let tint_position = Vec2::new(player.translation.x, player.translation.z);

    for (mut held, mut held_transform, mut held_visibility) in &mut roots {
        let block_changed = held.set_block_id(selected_block_id);

        if *held_visibility != visibility {
            *held_visibility = visibility;
        }

        let Some(block_id) = held.block_id() else {
            continue;
        };
        let block = content
            .blocks
            .get(block_id)
            .unwrap_or_else(|| panic!("hotbar references missing block: {block_id}"));
        let orientation = if Some(block_id) == hotbar_block_id {
            content
                .placement_orientation
                .for_block(selected_slot, block)
        } else {
            block.default_orientation()
        };
        held_transform.rotation = held_block_transform(orientation).rotation;

        let tint = block_tint_at(
            block.tint,
            tint_position,
            &content.biome_field,
            &content.biomes,
        );

        for (face, material_handle, mut layer_visibility) in &mut faces {
            let Some(mut material) = materials.get_mut(&material_handle.0) else {
                continue;
            };

            if block_changed {
                if let Some(face_material) = block_face_material_data(
                    face.face,
                    face.layer_index,
                    block,
                    &content.asset_server,
                    held.opacity(),
                ) {
                    *material = face_material;
                    apply_block_display_shading(&mut material, face.face, held.opacity());
                    *layer_visibility = Visibility::Visible;
                } else {
                    *layer_visibility = Visibility::Hidden;
                    continue;
                }
            }

            set_block_model_tint(&mut material, tint);
        }
    }
}

fn item_visibility(block_id: Option<&'static str>) -> Visibility {
    if block_id.is_some() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

fn held_block_transform(orientation: BlockOrientation) -> Transform {
    let viewmodel_rotation = base_viewmodel_transform().rotation;
    Transform::from_translation(Vec3::new(-0.02, ARM_SIZE.y + 0.04, 0.20))
        .with_rotation(viewmodel_rotation.inverse() * orientation_rotation(orientation))
        .with_scale(Vec3::splat(HELD_BLOCK_SCALE))
}
