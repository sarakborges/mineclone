use std::f32::consts::PI;

use bevy::{ecs::system::SystemParam, light::NotShadowCaster, prelude::*};

use crate::{
    app::game_state::GameState,
    content::{biome::BiomeRegistry, block::BlockRegistry},
    rendering::{
        block_model::{
            block_face_material, block_face_material_data, block_face_mesh, block_faces,
        },
        block_tint::block_tint_at,
    },
    voxel::mesh::BlockFace,
    world::biome_field::BiomeField,
};

use super::{camera::GameplayCamera, hotbar::PlayerHotbar};

const ARM_SIZE: Vec3 = Vec3::new(0.16, 0.56, 0.16);
const HELD_BLOCK_SCALE: f32 = 0.16;
const BREAK_ANIMATION_DURATION: f32 = 0.22;
const PLACE_ANIMATION_DURATION: f32 = 0.16;

#[derive(Component)]
struct PlayerViewModel;

#[derive(Component)]
struct HeldBlockRoot {
    block_id: Option<&'static str>,
}

#[derive(Component)]
struct HeldBlockFace {
    face: BlockFace,
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
        app.init_resource::<ViewModelAnimation>().add_systems(
            Update,
            (spawn_viewmodel, sync_held_block, animate_viewmodel)
                .chain()
                .run_if(in_state(GameState::Gameplay)),
        );
    }
}

fn spawn_viewmodel(
    mut commands: Commands,
    cameras: Query<(Entity, &Transform), Added<GameplayCamera>>,
    content: ViewModelContent,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (camera, camera_transform) in &cameras {
        let arm_mesh = meshes.add(Cuboid::new(ARM_SIZE.x, ARM_SIZE.y, ARM_SIZE.z));
        let arm_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.72, 0.52, 0.40),
            perceptual_roughness: 1.0,
            unlit: true,
            ..default()
        });
        let selected_block_id = content.hotbar.item_at(content.hotbar.selected_slot());
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
                    viewmodel
                        .spawn((
                            Mesh3d(arm_mesh.clone()),
                            MeshMaterial3d(arm_material.clone()),
                            Transform::from_translation(Vec3::new(0.0, ARM_SIZE.y * 0.5, 0.0)),
                            NotShadowCaster,
                        ))
                        .with_children(|arm| {
                            arm.spawn((
                                HeldBlockRoot {
                                    block_id: selected_block_id,
                                },
                                Transform::from_translation(Vec3::new(
                                    -0.015,
                                    ARM_SIZE.y * 0.5 + 0.06,
                                    -0.11,
                                ))
                                .with_rotation(Quat::from_euler(EulerRot::XYZ, 0.12, -0.62, -0.06))
                                .with_scale(Vec3::splat(HELD_BLOCK_SCALE)),
                                if selected_block_id.is_some() {
                                    Visibility::Visible
                                } else {
                                    Visibility::Hidden
                                },
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

                                for face in block_faces() {
                                    let mesh = meshes.add(block_face_mesh(face));
                                    let material = block_face_material(
                                        face,
                                        block,
                                        &content.asset_server,
                                        &mut materials,
                                        1.0,
                                    );
                                    if let Some(mut face_material) = materials.get_mut(&material) {
                                        face_material.base_color = tint;
                                    }

                                    held.spawn((
                                        HeldBlockFace { face },
                                        Mesh3d(mesh),
                                        MeshMaterial3d(material),
                                        NotShadowCaster,
                                    ));
                                }
                            });
                        });
                });
        });
    }
}

fn sync_held_block(
    content: ViewModelContent,
    player: Single<&Transform, With<GameplayCamera>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut roots: Query<(&mut HeldBlockRoot, &mut Visibility)>,
    faces: Query<(&HeldBlockFace, &MeshMaterial3d<StandardMaterial>)>,
) {
    let selected_block_id = content.hotbar.item_at(content.hotbar.selected_slot());
    let tint_position = Vec2::new(player.translation.x, player.translation.z);

    for (mut held, mut visibility) in &mut roots {
        let block_changed = held.block_id != selected_block_id;

        if block_changed {
            held.block_id = selected_block_id;
        }

        let Some(block_id) = selected_block_id else {
            *visibility = Visibility::Hidden;
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

        *visibility = Visibility::Visible;

        for (face, material_handle) in &faces {
            let Some(mut material) = materials.get_mut(&material_handle.0) else {
                continue;
            };

            if block_changed {
                *material =
                    block_face_material_data(face.face, block, &content.asset_server, 1.0);
            }

            if material.base_color != tint {
                material.base_color = tint;
            }
        }
    }
}

fn animate_viewmodel(
    time: Res<Time>,
    mut animation: ResMut<ViewModelAnimation>,
    mut viewmodels: Query<&mut Transform, With<PlayerViewModel>>,
) {
    let Some(action) = animation.action else {
        return;
    };

    animation.elapsed += time.delta_secs();
    let duration = match action {
        ViewModelAction::Break => BREAK_ANIMATION_DURATION,
        ViewModelAction::Place => PLACE_ANIMATION_DURATION,
    };
    let progress = (animation.elapsed / duration).clamp(0.0, 1.0);
    let wave = (progress * PI).sin();

    for mut transform in &mut viewmodels {
        let mut animated = base_viewmodel_transform();

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

        *transform = animated;
    }

    if progress >= 1.0 {
        animation.action = None;
        animation.elapsed = 0.0;

        for mut transform in &mut viewmodels {
            *transform = base_viewmodel_transform();
        }
    }
}

fn base_viewmodel_transform() -> Transform {
    Transform::from_translation(Vec3::new(0.64, -0.78, -1.12)).with_rotation(Quat::from_euler(
        EulerRot::XYZ,
        -0.22,
        -0.10,
        0.28,
    ))
}
