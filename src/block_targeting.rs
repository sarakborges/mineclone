use bevy::prelude::*;

use crate::{game_state::GameState, voxel_chunk::VoxelChunk};

const TARGET_RANGE: f32 = 5.0;
const HIGHLIGHT_SCALE: f32 = 1.01;

pub struct BlockTargetingPlugin;

impl Plugin for BlockTargetingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), setup_targeting)
            .add_systems(
                Update,
                update_targeting.run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Component)]
struct TargetHighlight;

#[derive(Component)]
struct TargetBlockText;

fn setup_targeting(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(HIGHLIGHT_SCALE)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, 0.18),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::default(),
        Visibility::Hidden,
        TargetHighlight,
        DespawnOnExit(GameState::Gameplay),
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        Pickable::IGNORE,
        DespawnOnExit(GameState::Gameplay),
        children![(
            Text::new("+"),
            TextFont {
                font_size: FontSize::Px(24.0),
                ..default()
            },
            TextColor(Color::WHITE),
        )],
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(16),
            right: px(16),
            padding: UiRect::all(px(12)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.025, 0.04, 0.82)),
        BorderRadius::all(px(6)),
        Pickable::IGNORE,
        DespawnOnExit(GameState::Gameplay),
        children![(
            Text::new("No block targeted"),
            TextFont {
                font_size: FontSize::Px(18.0),
                ..default()
            },
            TextColor(Color::WHITE),
            TargetBlockText,
        )],
    ));
}

fn update_targeting(
    camera: Single<&GlobalTransform, With<Camera3d>>,
    chunk: Single<&VoxelChunk>,
    mut highlight: Single<(&mut Transform, &mut Visibility), With<TargetHighlight>>,
    mut target_text: Single<&mut Text, With<TargetBlockText>>,
) {
    let origin = camera.translation();
    let direction = camera.forward().as_vec3();

    if let Some(hit) = raycast_voxels(&chunk, origin, direction, TARGET_RANGE) {
        highlight.0.translation = hit.voxel.as_vec3() + Vec3::splat(0.5);
        *highlight.1 = Visibility::Visible;
        **target_text = format!(
            "{}\n({}, {}, {})",
            hit.block_id, hit.voxel.x, hit.voxel.y, hit.voxel.z
        );
    } else {
        *highlight.1 = Visibility::Hidden;
        **target_text = "No block targeted".to_string();
    }
}

struct VoxelHit {
    voxel: IVec3,
    block_id: &'static str,
}

fn raycast_voxels(
    chunk: &VoxelChunk,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<VoxelHit> {
    if direction.length_squared() == 0.0 {
        return None;
    }

    let direction = direction.normalize();
    let mut voxel = origin.floor().as_ivec3();
    let step = IVec3::new(
        direction.x.signum() as i32,
        direction.y.signum() as i32,
        direction.z.signum() as i32,
    );

    let t_delta = Vec3::new(
        reciprocal_abs(direction.x),
        reciprocal_abs(direction.y),
        reciprocal_abs(direction.z),
    );
    let mut t_max = Vec3::new(
        first_boundary_distance(origin.x, voxel.x, direction.x),
        first_boundary_distance(origin.y, voxel.y, direction.y),
        first_boundary_distance(origin.z, voxel.z, direction.z),
    );
    let mut distance = 0.0;

    loop {
        if let Some(block_id) = chunk.block_id_at(voxel.x, voxel.y, voxel.z) {
            return Some(VoxelHit { voxel, block_id });
        }

        if t_max.x <= t_max.y && t_max.x <= t_max.z {
            distance = t_max.x;
            voxel.x += step.x;
            t_max.x += t_delta.x;
        } else if t_max.y <= t_max.z {
            distance = t_max.y;
            voxel.y += step.y;
            t_max.y += t_delta.y;
        } else {
            distance = t_max.z;
            voxel.z += step.z;
            t_max.z += t_delta.z;
        }

        if distance > max_distance {
            return None;
        }
    }
}

fn reciprocal_abs(value: f32) -> f32 {
    if value == 0.0 {
        f32::INFINITY
    } else {
        1.0 / value.abs()
    }
}

fn first_boundary_distance(origin: f32, voxel: i32, direction: f32) -> f32 {
    if direction > 0.0 {
        (voxel as f32 + 1.0 - origin) / direction
    } else if direction < 0.0 {
        (origin - voxel as f32) / -direction
    } else {
        f32::INFINITY
    }
}
