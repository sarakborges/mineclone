use bevy::{
    asset::RenderAssetUsages,
    light::NotShadowCaster,
    mesh::Indices,
    prelude::*,
    render::render_resource::PrimitiveTopology,
};

use crate::{
    app::game_state::GameState,
    content::block::{BlockDefinition, BlockRegistry},
    voxel::mesh::BlockFace,
};

use super::{camera::GameplayCamera, hotbar::PlayerHotbar};

const ARM_SIZE: Vec3 = Vec3::new(0.18, 0.64, 0.18);
const HELD_BLOCK_SCALE: f32 = 0.30;

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

pub struct PlayerViewModelPlugin;

impl Plugin for PlayerViewModelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (spawn_viewmodel, sync_held_block)
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
                    Transform::from_translation(Vec3::new(0.53, -0.46, -0.86))
                        .with_rotation(Quat::from_euler(EulerRot::XYZ, -0.18, -0.18, 0.10)),
                    Visibility::Visible,
                ))
                .with_children(|viewmodel| {
                    viewmodel.spawn((
                        Mesh3d(arm_mesh.clone()),
                        MeshMaterial3d(arm_material.clone()),
                        Transform::from_translation(Vec3::new(0.10, -0.19, 0.05))
                            .with_rotation(Quat::from_rotation_z(-0.22)),
                        NotShadowCaster,
                    ));

                    viewmodel
                        .spawn((
                            HeldBlockRoot {
                                block_id: selected_block_id,
                            },
                            Transform::from_translation(Vec3::new(-0.08, 0.15, -0.15))
                                .with_rotation(Quat::from_euler(EulerRot::XYZ, 0.16, -0.52, 0.04))
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
            );
        }
    }
}

fn block_face_material(
    face: BlockFace,
    block: &BlockDefinition,
    asset_server: &AssetServer,
    materials: &mut Assets<StandardMaterial>,
) -> Handle<StandardMaterial> {
    let texture = match face {
        BlockFace::Right => &block.textures.right,
        BlockFace::Left => &block.textures.left,
        BlockFace::Top => &block.textures.top,
        BlockFace::Bottom => &block.textures.bottom,
        BlockFace::Front => &block.textures.front,
        BlockFace::Back => &block.textures.back,
    };

    materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(asset_server.load(texture.clone())),
        perceptual_roughness: 1.0,
        unlit: true,
        ..default()
    })
}

fn block_faces() -> [BlockFace; 6] {
    [
        BlockFace::Right,
        BlockFace::Left,
        BlockFace::Top,
        BlockFace::Bottom,
        BlockFace::Front,
        BlockFace::Back,
    ]
}

fn block_face_mesh(face: BlockFace) -> Mesh {
    let (vertices, normal) = match face {
        BlockFace::Right => (
            [[0.5, -0.5, 0.5], [0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [0.5, 0.5, 0.5]],
            [1.0, 0.0, 0.0],
        ),
        BlockFace::Left => (
            [[-0.5, -0.5, -0.5], [-0.5, -0.5, 0.5], [-0.5, 0.5, 0.5], [-0.5, 0.5, -0.5]],
            [-1.0, 0.0, 0.0],
        ),
        BlockFace::Top => (
            [[-0.5, 0.5, 0.5], [0.5, 0.5, 0.5], [0.5, 0.5, -0.5], [-0.5, 0.5, -0.5]],
            [0.0, 1.0, 0.0],
        ),
        BlockFace::Bottom => (
            [[-0.5, -0.5, -0.5], [0.5, -0.5, -0.5], [0.5, -0.5, 0.5], [-0.5, -0.5, 0.5]],
            [0.0, -1.0, 0.0],
        ),
        BlockFace::Front => (
            [[-0.5, -0.5, 0.5], [0.5, -0.5, 0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5]],
            [0.0, 0.0, 1.0],
        ),
        BlockFace::Back => (
            [[0.5, -0.5, -0.5], [-0.5, -0.5, -0.5], [-0.5, 0.5, -0.5], [0.5, 0.5, -0.5]],
            [0.0, 0.0, -1.0],
        ),
    };

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices.to_vec())
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![normal; 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
    )
    .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
}
