use std::f32::consts::PI;

use bevy::{light::NotShadowCaster, prelude::*};

use crate::{
    app::game_state::GameState,
    content::block::BlockRegistry,
    rendering::block_model::{block_face_material, block_face_mesh, block_faces},
    voxel::mesh::BlockFace,
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
    cameras: Query<Entity, Added<GameplayCamera>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    blocks: Res<BlockRegistry>,
    hotbar: Res<PlayerHotbar>,
) {
    for camera in &cameras {
        let arm_mesh = meshes.add(Cuboid::new(ARM_SIZE.x, ARM_SIZE.y, ARM_SIZE.z));
        let arm_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.72, 0.52, 0.40),
            perceptual_roughness: 1.0,
            unlit: true,
            ..default()
        });
        let selected_block_id = hotbar.item_at(hotbar.selected_slot());

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
                                    -0.02,
                                    ARM_SIZE.y * 0.5 + 0.16,
                                    -0.18,
                                ))
                                .with_rotation(Quat::from_euler(
                                    EulerRot::XYZ,
                                    0.12,
                                    -0.62,
                                    -0.06,
                                ))
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
                                let block = blocks.get(block_id).unwrap_or_else(|| {
                                    panic!("hotbar references missing block: {block_id}")
                                });

                                for face in block_faces() {
                                    let mesh = meshes.add(block_face_mesh(face));
                                    let material = block_face_material(
                                        face,
                                        block,
                                        &asset_server,
                                        &mut materials,
                                        1.0,
                                    );

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
    hotbar: Res<PlayerHotbar>,
    blocks: Res<BlockRegistry>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut roots: Query<(&mut HeldBlockRoot, &mut Visibility)>,
    mut faces: Query<(&HeldBlockFace, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    if !hotbar.is_changed() {
        return;
    }

    let selected_block_id = hotbar.item_at(hotbar.selected_slot());

    for (mut held, mut visibility) in &mut roots {
        if held.block_id == selected_block_id {
            continue;
        }

        held.block_id = selected_block_id;

        let Some(block_id) = selected_block_id else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let block = blocks
            .get(block_id)
            .unwrap_or_else(|| panic!("hotbar references missing block: {block_id}"));

        *visibility = Visibility::Visible;

        for (face, mut material) in &mut faces {
            material.0 = block_face_material(
                face.face,
                block,
                &asset_server,
                &mut materials,
                1.0,
            );
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
                animated.rotation *= Quat::from_euler(
                    EulerRot::XYZ,
                    -0.68 * wave,
                    0.12 * wave,
                    -0.34 * wave,
                );
            }
            ViewModelAction::Place => {
                animated.translation += Vec3::new(-0.03, 0.01, -0.14) * wave;
                animated.rotation *= Quat::from_euler(
                    EulerRot::XYZ,
                    -0.18 * wave,
                    0.05 * wave,
                    -0.08 * wave,
                );
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
    Transform::from_translation(Vec3::new(0.64, -0.78, -1.12))
        .with_rotation(Quat::from_euler(EulerRot::XYZ, -0.22, -0.10, 0.28))
}
