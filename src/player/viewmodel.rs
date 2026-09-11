use std::f32::consts::PI;

use bevy::{ecs::system::SystemParam, light::NotShadowCaster, prelude::*};

use crate::{
    app::game_state::GameState,
    content::{biome::BiomeRegistry, block::BlockRegistry},
    rendering::{
        block_model::{
            BlockModel, BlockModelMaterials, BlockModelMeshes, apply_block_display_shading,
            block_face_material_data, set_block_model_tint,
        },
        block_model_material::BlockModelMaterial,
        block_tint::block_tint_at,
    },
    voxel::mesh::BlockFace,
    world::biome_field::BiomeField,
};

use super::{camera::GameplayCamera, hotbar::PlayerHotbar};

const ARM_SIZE: Vec3 = Vec3::new(0.23, 0.60, 0.21);
const HELD_BLOCK_SCALE: f32 = 0.18;
const BREAK_ANIMATION_DURATION: f32 = 0.16;
const PLACE_ANIMATION_DURATION: f32 = 0.22;
const ITEM_SWITCH_ANIMATION_DURATION: f32 = 0.30;

#[derive(Component)]
struct PlayerViewModel;

#[derive(Component)]
struct ViewModelArm;

#[derive(Component)]
struct HeldBlockRoot;

#[derive(Component)]
struct HeldBlockFace {
    face: BlockFace,
}

type ArmVisibilityQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Visibility,
    (
        With<ViewModelArm>,
        Without<HeldBlockRoot>,
        Without<PlayerViewModel>,
    ),
>;

type HeldBlockRootQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut BlockModel, &'static mut Visibility),
    (
        With<HeldBlockRoot>,
        Without<ViewModelArm>,
        Without<PlayerViewModel>,
    ),
>;

#[derive(Resource)]
struct ViewModelArmAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

#[derive(Clone, Copy)]
enum ViewModelAction {
    Break,
    Place,
}

#[derive(Resource, Default)]
pub(crate) struct ViewModelAnimation {
    action: Option<ViewModelAction>,
    elapsed: f32,
}

impl ViewModelAnimation {
    pub(crate) fn play_break(&mut self) {
        self.action = Some(ViewModelAction::Break);
        self.elapsed = 0.0;
    }

    pub(crate) fn play_place(&mut self) {
        self.action = Some(ViewModelAction::Place);
        self.elapsed = 0.0;
    }
}

#[derive(Resource, Default)]
struct ViewModelItemSwitch {
    initialized: bool,
    displayed_block_id: Option<&'static str>,
    target_block_id: Option<&'static str>,
    elapsed: f32,
    active: bool,
}

#[derive(SystemParam)]
struct ViewModelContent<'w> {
    asset_server: Res<'w, AssetServer>,
    blocks: Res<'w, BlockRegistry>,
    biomes: Res<'w, BiomeRegistry>,
    biome_field: Res<'w, BiomeField>,
    hotbar: Res<'w, PlayerHotbar>,
}

pub struct PlayerViewModelPlugin;

impl Plugin for PlayerViewModelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewModelAnimation>()
            .init_resource::<ViewModelItemSwitch>()
            .add_systems(Startup, setup_viewmodel_arm_assets)
            .add_systems(
                Update,
                (
                    spawn_viewmodel,
                    advance_item_switch,
                    sync_held_block,
                    animate_viewmodel,
                )
                    .chain()
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn setup_viewmodel_arm_assets(
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

fn spawn_viewmodel(
    mut commands: Commands,
    cameras: Query<(Entity, &Transform), Added<GameplayCamera>>,
    content: ViewModelContent,
    block_meshes: Res<BlockModelMeshes>,
    block_materials: Res<BlockModelMaterials>,
    arm_assets: Res<ViewModelArmAssets>,
    mut item_switch: ResMut<ViewModelItemSwitch>,
    mut materials: ResMut<Assets<BlockModelMaterial>>,
) {
    for (camera, camera_transform) in &cameras {
        let selected_block_id = content.hotbar.item_at(content.hotbar.selected_slot());
        item_switch.initialized = true;
        item_switch.displayed_block_id = selected_block_id;
        item_switch.target_block_id = selected_block_id;
        item_switch.elapsed = 0.0;
        item_switch.active = false;

        let item_visibility = item_visibility(selected_block_id);
        let tint_position = Vec2::new(
            camera_transform.translation.x,
            camera_transform.translation.z,
        );
        let block_model = selected_block_id
            .map(BlockModel::display)
            .unwrap_or_else(BlockModel::empty_display);

        commands.entity(camera).with_children(|camera| {
            camera
                .spawn((
                    PlayerViewModel,
                    base_viewmodel_transform(),
                    Visibility::Visible,
                    DespawnOnExit(GameState::Gameplay),
                ))
                .with_children(|viewmodel| {
                    viewmodel.spawn((
                        Mesh3d(arm_assets.mesh.clone()),
                        MeshMaterial3d(arm_assets.material.clone()),
                        ViewModelArm,
                        Transform::from_translation(Vec3::new(0.31, -0.31, -0.57))
                            .with_rotation(Quat::from_rotation_z(-0.16)),
                        item_visibility.arm,
                        NotShadowCaster,
                    ));

                    let held = viewmodel
                        .spawn((
                            HeldBlockRoot,
                            block_model,
                            Transform::from_translation(Vec3::new(0.33, -0.24, -0.64))
                                .with_rotation(
                                    Quat::from_rotation_x(-0.22)
                                        * Quat::from_rotation_y(-0.58)
                                        * Quat::from_rotation_z(0.08),
                                )
                                .with_scale(Vec3::splat(HELD_BLOCK_SCALE)),
                            item_visibility.block,
                            NotShadowCaster,
                        ))
                        .id();

                    commands.entity(held).with_children(|held| {
                        for face in BlockModel::display(selected_block_id.unwrap_or_default()).faces()
                        {
                            let mesh = block_meshes.face(*face).clone();
                            let material = selected_block_id
                                .and_then(|block_id| content.blocks.get(block_id).map(|block| (block_id, block)))
                                .map(|(block_id, block)| {
                                    let tint = block_tint_at(
                                        block.tint,
                                        tint_position,
                                        &content.biome_field,
                                        &content.biomes,
                                    );
                                    let mut material = block_face_material_data(
                                        *face,
                                        block,
                                        &content.asset_server,
                                        1.0,
                                    );
                                    set_block_model_tint(&mut material, tint);
                                    apply_block_display_shading(&mut material, *face, 1.0);
                                    materials.add(material)
                                })
                                .unwrap_or_else(|| block_materials.placeholder().clone());

                            held.spawn((
                                Mesh3d(mesh),
                                MeshMaterial3d(material),
                                HeldBlockFace { face: *face },
                                NotShadowCaster,
                            ));
                        }
                    });
                });
        });
    }
}

#[derive(Clone, Copy)]
struct ItemVisibility {
    arm: Visibility,
    block: Visibility,
}

fn item_visibility(block_id: Option<&str>) -> ItemVisibility {
    if block_id.is_some() {
        ItemVisibility {
            arm: Visibility::Hidden,
            block: Visibility::Visible,
        }
    } else {
        ItemVisibility {
            arm: Visibility::Visible,
            block: Visibility::Hidden,
        }
    }
}

fn sync_held_block(
    cameras: Query<&Transform, With<GameplayCamera>>,
    content: ViewModelContent,
    mut held_roots: HeldBlockRootQuery,
    faces: Query<(&HeldBlockFace, &MeshMaterial3d<BlockModelMaterial>)>,
    mut materials: ResMut<Assets<BlockModelMaterial>>,
    mut item_switch: ResMut<ViewModelItemSwitch>,
) {
    let selected_block_id = content.hotbar.item_at(content.hotbar.selected_slot());
    if !item_switch.initialized {
        item_switch.initialized = true;
        item_switch.displayed_block_id = selected_block_id;
        item_switch.target_block_id = selected_block_id;
    } else if item_switch.target_block_id != selected_block_id {
        item_switch.target_block_id = selected_block_id;
        item_switch.elapsed = 0.0;
        item_switch.active = true;
    }

    let displayed_block_id = item_switch.displayed_block_id;
    let visibility = item_visibility(displayed_block_id);
    for (mut model, mut root_visibility) in &mut held_roots {
        model.set_block_id(displayed_block_id);
        *root_visibility = visibility.block;
    }

    let Ok(camera_transform) = cameras.single() else {
        return;
    };
    let tint_position = Vec2::new(
        camera_transform.translation.x,
        camera_transform.translation.z,
    );

    for (face, material_handle) in &faces {
        let Some(material) = materials.get_mut(&material_handle.0) else {
            continue;
        };
        let Some(block_id) = displayed_block_id else {
            continue;
        };
        let block = content
            .blocks
            .get(block_id)
            .unwrap_or_else(|| panic!("missing block definition: {block_id}"));
        let tint = block_tint_at(
            block.tint,
            tint_position,
            &content.biome_field,
            &content.biomes,
        );
        *material = block_face_material_data(face.face, block, &content.asset_server, 1.0);
        set_block_model_tint(material, tint);
        apply_block_display_shading(material, face.face, 1.0);
    }
}

fn advance_item_switch(
    time: Res<Time>,
    mut switch: ResMut<ViewModelItemSwitch>,
) {
    if !switch.active {
        return;
    }

    switch.elapsed += time.delta_secs();
    let progress = (switch.elapsed / ITEM_SWITCH_ANIMATION_DURATION).clamp(0.0, 1.0);

    if progress >= 0.5 && switch.displayed_block_id != switch.target_block_id {
        switch.displayed_block_id = switch.target_block_id;
    }
    if progress >= 1.0 {
        switch.active = false;
        switch.elapsed = 0.0;
    }
}

fn animate_viewmodel(
    time: Res<Time>,
    mut animation: ResMut<ViewModelAnimation>,
    switch: Res<ViewModelItemSwitch>,
    mut viewmodels: Query<&mut Transform, With<PlayerViewModel>>,
    mut arms: ArmVisibilityQuery,
) {
    let Ok(mut transform) = viewmodels.single_mut() else {
        return;
    };

    let mut action_offset = Vec3::ZERO;
    let mut action_rotation = Quat::IDENTITY;
    if let Some(action) = animation.action {
        animation.elapsed += time.delta_secs();
        let duration = match action {
            ViewModelAction::Break => BREAK_ANIMATION_DURATION,
            ViewModelAction::Place => PLACE_ANIMATION_DURATION,
        };
        let progress = (animation.elapsed / duration).clamp(0.0, 1.0);
        let arc = (progress * PI).sin();

        match action {
            ViewModelAction::Break => {
                action_offset = Vec3::new(-0.08 * arc, -0.03 * arc, 0.10 * arc);
                action_rotation = Quat::from_rotation_x(-0.75 * arc)
                    * Quat::from_rotation_z(0.32 * arc);
            }
            ViewModelAction::Place => {
                action_offset = Vec3::new(0.05 * arc, 0.02 * arc, -0.11 * arc);
                action_rotation = Quat::from_rotation_x(0.48 * arc)
                    * Quat::from_rotation_z(-0.18 * arc);
            }
        }

        if progress >= 1.0 {
            animation.action = None;
            animation.elapsed = 0.0;
        }
    }

    let switch_offset = if switch.active {
        let progress = (switch.elapsed / ITEM_SWITCH_ANIMATION_DURATION).clamp(0.0, 1.0);
        let dip = (progress * PI).sin();
        Vec3::new(0.03 * dip, -0.27 * dip, 0.08 * dip)
    } else {
        Vec3::ZERO
    };

    *transform = base_viewmodel_transform();
    transform.translation += action_offset + switch_offset;
    transform.rotation = transform.rotation * action_rotation;

    let selected_visibility = item_visibility(switch.displayed_block_id);
    for mut visibility in &mut arms {
        *visibility = selected_visibility.arm;
    }
}

fn base_viewmodel_transform() -> Transform {
    Transform::IDENTITY
}
