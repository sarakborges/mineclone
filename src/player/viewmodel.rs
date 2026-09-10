use std::f32::consts::PI;

use bevy::{ecs::system::SystemParam, light::NotShadowCaster, prelude::*};

use crate::{
    app::game_state::GameState,
    content::{biome::BiomeRegistry, block::BlockRegistry},
    rendering::{
        block_model::{
            BlockModelMaterials, BlockModelMeshes, block_face_material_data, set_block_model_tint,
        },
        block_model_material::BlockModelMaterial,
        block_tint::block_tint_at,
    },
    voxel::mesh::BlockFace,
    world::biome_field::BiomeField,
};

use super::{camera::GameplayCamera, hotbar::PlayerHotbar};

const ARM_SIZE: Vec3 = Vec3::new(0.30, 0.60, 0.28);
const HELD_BLOCK_SCALE: f32 = 0.18;
const BREAK_ANIMATION_DURATION: f32 = 0.22;
const PLACE_ANIMATION_DURATION: f32 = 0.16;
const ITEM_SWITCH_ANIMATION_DURATION: f32 = 0.30;

#[derive(Component)]
struct PlayerViewModel;

#[derive(Component)]
struct ViewModelArm;

#[derive(Component)]
struct HeldBlockRoot {
    block_id: Option<&'static str>,
}

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
    (&'static mut HeldBlockRoot, &'static mut Visibility),
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

        commands.entity(camera).with_children(|camera| {
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
                        item_visibility,
                        NotShadowCaster,
                    ));

                    viewmodel
                        .spawn((
                            HeldBlockRoot {
                                block_id: selected_block_id,
                            },
                            held_block_transform(),
                            item_visibility,
                        ))
                        .with_children(|held| {
                            let Some(block_id) = selected_block_id else {
                                return;
                            };
                            let block = content.blocks.get(block_id).unwrap_or_else(|| {
                                panic!("hotbar references missing block: {block_id}")
                            });
                            let tint = block_tint_at(
                                block_id,
                                tint_position,
                                &content.biome_field,
                                &content.biomes,
                            );

                            for face in held_block_faces() {
                                let material = block_materials.held_for_face(face);
                                let Some(mut face_material) = materials.get_mut(&material) else {
                                    continue;
                                };
                                *face_material = block_face_material_data(
                                    face,
                                    block,
                                    &content.asset_server,
                                    1.0,
                                );
                                apply_held_face_shading(&mut face_material, face);
                                set_block_model_tint(&mut face_material, tint);

                                held.spawn((
                                    HeldBlockFace { face },
                                    Mesh3d(block_meshes.for_face(face)),
                                    MeshMaterial3d(material),
                                    NotShadowCaster,
                                ));
                            }
                        });
                });
        });
    }
}

fn advance_item_switch(
    time: Res<Time>,
    hotbar: Res<PlayerHotbar>,
    mut item_switch: ResMut<ViewModelItemSwitch>,
) {
    let selected_block_id = hotbar.item_at(hotbar.selected_slot());

    if !item_switch.initialized {
        item_switch.initialized = true;
        item_switch.displayed_block_id = selected_block_id;
        item_switch.target_block_id = selected_block_id;
        return;
    }

    if selected_block_id != item_switch.target_block_id {
        item_switch.target_block_id = selected_block_id;
        item_switch.elapsed = 0.0;
        item_switch.active = true;
    }

    if !item_switch.active {
        return;
    }

    item_switch.elapsed += time.delta_secs();
    let midpoint = ITEM_SWITCH_ANIMATION_DURATION * 0.5;

    if item_switch.elapsed >= midpoint
        && item_switch.displayed_block_id != item_switch.target_block_id
    {
        item_switch.displayed_block_id = item_switch.target_block_id;
    }

    if item_switch.elapsed >= ITEM_SWITCH_ANIMATION_DURATION {
        item_switch.displayed_block_id = item_switch.target_block_id;
        item_switch.elapsed = 0.0;
        item_switch.active = false;
    }
}

fn sync_held_block(
    content: ViewModelContent,
    item_switch: Res<ViewModelItemSwitch>,
    player: Single<&Transform, With<GameplayCamera>>,
    mut materials: ResMut<Assets<BlockModelMaterial>>,
    mut arms: ArmVisibilityQuery,
    mut roots: HeldBlockRootQuery,
    faces: Query<(&HeldBlockFace, &MeshMaterial3d<BlockModelMaterial>)>,
) {
    let selected_block_id = item_switch.displayed_block_id;
    let visibility = item_visibility(selected_block_id);
    let tint_position = Vec2::new(player.translation.x, player.translation.z);

    for mut arm_visibility in &mut arms {
        if *arm_visibility != visibility {
            *arm_visibility = visibility;
        }
    }

    for (mut held, mut held_visibility) in &mut roots {
        let block_changed = held.block_id != selected_block_id;

        if block_changed {
            held.block_id = selected_block_id;
        }
        if *held_visibility != visibility {
            *held_visibility = visibility;
        }

        let Some(block_id) = selected_block_id else {
            continue;
        };
        let block = content
            .blocks
            .get(block_id)
            .unwrap_or_else(|| panic!("hotbar references missing block: {block_id}"));
        let tint = block_tint_at(
            block_id,
            tint_position,
            &content.biome_field,
            &content.biomes,
        );

        for (face, material_handle) in &faces {
            let Some(mut material) = materials.get_mut(&material_handle.0) else {
                continue;
            };

            if block_changed {
                *material = block_face_material_data(face.face, block, &content.asset_server, 1.0);
                apply_held_face_shading(&mut material, face.face);
            }

            set_block_model_tint(&mut material, tint);
        }
    }
}

fn animate_viewmodel(
    time: Res<Time>,
    mut animation: ResMut<ViewModelAnimation>,
    item_switch: Res<ViewModelItemSwitch>,
    mut viewmodels: Query<&mut Transform, With<PlayerViewModel>>,
) {
    let interaction = animation.action.and_then(|action| {
        animation.elapsed += time.delta_secs();
        let duration = match action {
            ViewModelAction::Break => BREAK_ANIMATION_DURATION,
            ViewModelAction::Place => PLACE_ANIMATION_DURATION,
        };
        let progress = (animation.elapsed / duration).clamp(0.0, 1.0);

        if progress >= 1.0 {
            animation.action = None;
            animation.elapsed = 0.0;
            None
        } else {
            Some((action, (progress * PI).sin()))
        }
    });

    let switch_wave = if item_switch.active {
        let progress =
            (item_switch.elapsed / ITEM_SWITCH_ANIMATION_DURATION).clamp(0.0, 1.0);
        (progress * PI).sin()
    } else {
        0.0
    };

    for mut transform in &mut viewmodels {
        let mut animated = base_viewmodel_transform();

        if let Some((action, wave)) = interaction {
            match action {
                ViewModelAction::Break => {
                    animated.translation += Vec3::new(-0.06, -0.10, -0.06) * wave;
                    animated.rotation *=
                        Quat::from_euler(EulerRot::XYZ, -0.68 * wave, 0.12 * wave, -0.34 * wave);
                }
                ViewModelAction::Place => {
                    animated.translation += Vec3::new(-0.03, 0.01, -0.14) * wave;
                    animated.rotation *=
                        Quat::from_euler(EulerRot::XYZ, -0.18 * wave, 0.05 * wave, -0.08 * wave);
                }
            }
        }

        if switch_wave > 0.0 {
            animated.translation += Vec3::new(0.10, -0.54, 0.14) * switch_wave;
            animated.rotation *= Quat::from_euler(
                EulerRot::XYZ,
                0.34 * switch_wave,
                0.0,
                0.16 * switch_wave,
            );
        }

        *transform = animated;
    }
}

fn item_visibility(block_id: Option<&'static str>) -> Visibility {
    if block_id.is_some() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

fn held_block_faces() -> [BlockFace; 3] {
    [BlockFace::Top, BlockFace::Front, BlockFace::Right]
}

fn apply_held_face_shading(material: &mut BlockModelMaterial, face: BlockFace) {
    let shade = match face {
        BlockFace::Top => 1.0,
        BlockFace::Front => 0.86,
        BlockFace::Right => 0.74,
        _ => 1.0,
    };
    material.base.base_color = Color::srgba(shade, shade, shade, 1.0);
}

fn base_viewmodel_transform() -> Transform {
    Transform::from_translation(Vec3::new(0.62, -0.80, -1.05)).with_rotation(Quat::from_euler(
        EulerRot::XYZ,
        -0.22,
        -0.10,
        0.28,
    ))
}

fn held_block_transform() -> Transform {
    Transform::from_translation(Vec3::new(-0.02, ARM_SIZE.y + 0.04, 0.20))
        .with_rotation(Quat::from_euler(EulerRot::XYZ, 0.34, -0.64, 0.0))
        .with_scale(Vec3::splat(HELD_BLOCK_SCALE))
}
