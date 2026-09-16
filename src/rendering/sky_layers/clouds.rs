use bevy::{
    light::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
};

use crate::{
    app::game_state::GameState, player::camera::GameplayCamera,
    world::current_context::CurrentDimensionContext,
};

use super::{
    assets::CloudAssets,
    deterministic::{hash_signed, hash01},
    state::SkyLayerVisualState,
};

const MAX_CLOUDS: usize = 24;
const CLOUD_SPAN: f32 = 180.0;
const CLOUD_SPEED: f32 = 1.6;

#[derive(Component)]
pub(super) struct CloudPart {
    cloud_index: usize,
    base: Vec2,
    altitude_above_sea_level: f32,
}

pub(super) fn spawn_clouds(mut commands: Commands, assets: Res<CloudAssets>) {
    for cloud_index in 0..MAX_CLOUDS {
        let seed = cloud_index as u32;
        let base = Vec2::new(
            hash_signed(seed.wrapping_mul(17).wrapping_add(3)) * CLOUD_SPAN * 0.5,
            hash_signed(seed.wrapping_mul(29).wrapping_add(11)) * CLOUD_SPAN * 0.5,
        );
        let altitude_above_sea_level = 34.0 + hash01(seed.wrapping_mul(37).wrapping_add(5)) * 14.0;
        let width = 9.0 + hash01(seed.wrapping_mul(43).wrapping_add(7)) * 8.0;
        let depth = 4.0 + hash01(seed.wrapping_mul(53).wrapping_add(13)) * 5.0;

        // Previously each visible cloud contained three overlapping, alpha-blended
        // cuboids. One cloud mesh avoids the extra transparent draw calls and
        // overdraw now that clouds are correctly positioned above terrain.
        commands.spawn((
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(assets.material.clone()),
            Transform::from_scale(Vec3::new(width, 0.7, depth)),
            Visibility::Hidden,
            NotShadowCaster,
            NotShadowReceiver,
            CloudPart {
                cloud_index,
                base,
                altitude_above_sea_level,
            },
            DespawnOnExit(GameState::Gameplay),
        ));
    }
}

pub(super) fn cloud_presentation_needs_sync(
    visuals: Res<SkyLayerVisualState>,
    added_clouds: Query<(), Added<CloudPart>>,
) -> bool {
    visuals.is_changed() || !added_clouds.is_empty()
}

pub(super) fn sync_cloud_presentation(
    visuals: Res<SkyLayerVisualState>,
    assets: Res<CloudAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut clouds: Query<(&CloudPart, &mut Visibility)>,
) {
    let visible_count = (visuals.cloud_density * MAX_CLOUDS as f32).round() as usize;
    let [red, green, blue] = visuals.cloud_color.to_srgb();
    let color = Color::srgba(red, green, blue, 0.78);

    let color_changed = materials
        .get(&assets.material)
        .is_some_and(|material| material.base_color != color);
    if color_changed && let Some(mut material) = materials.get_mut(&assets.material) {
        material.base_color = color;
    }

    for (cloud, mut visibility) in &mut clouds {
        let next_visibility = if cloud.cloud_index < visible_count {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != next_visibility {
            *visibility = next_visibility;
        }
    }
}

pub(super) fn update_cloud_positions(
    time: Res<Time>,
    visuals: Res<SkyLayerVisualState>,
    dimension: CurrentDimensionContext,
    camera: Single<&GlobalTransform, With<GameplayCamera>>,
    mut clouds: Query<(&CloudPart, &mut Transform)>,
) {
    if visuals.cloud_density <= 0.0 {
        return;
    }
    let Some(dimension) = dimension.definition() else {
        return;
    };

    let visible_count = (visuals.cloud_density * MAX_CLOUDS as f32).round() as usize;
    let camera_position = camera.translation();
    let sea_level = dimension.sea_level as f32;
    // Wind is global. Keep it bounded so long sessions retain positioning precision.
    let drift = (time.elapsed_secs() * CLOUD_SPEED).rem_euclid(CLOUD_SPAN);

    for (cloud, mut transform) in &mut clouds {
        if cloud.cloud_index >= visible_count {
            continue;
        }

        // Recycle the finite pool into the nearest world-space tile. The camera
        // selects a tile but is never added to the cloud's position: walking or
        // flying within a tile does not drag clouds along with the player.
        let world_x = cloud_world_coordinate(cloud.base.x, drift, camera_position.x);
        let world_z = cloud_world_coordinate(cloud.base.y, 0.0, camera_position.z);
        let translation = Vec3::new(world_x, sea_level + cloud.altitude_above_sea_level, world_z);
        if transform.translation != translation {
            transform.translation = translation;
        }
    }
}

fn cloud_world_coordinate(base: f32, drift: f32, camera: f32) -> f32 {
    let world_position = base + drift;
    let nearest_tile = ((camera - world_position) / CLOUD_SPAN).round();
    world_position + nearest_tile * CLOUD_SPAN
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_camera_does_not_move_cloud_within_its_world_tile() {
        assert_eq!(cloud_world_coordinate(12.0, 8.0, 0.0), 20.0);
        assert_eq!(cloud_world_coordinate(12.0, 8.0, 30.0), 20.0);
    }

    #[test]
    fn distant_cloud_recycles_by_whole_world_tiles() {
        assert_eq!(cloud_world_coordinate(10.0, 0.0, 0.0), 10.0);
        assert_eq!(cloud_world_coordinate(10.0, 0.0, 200.0), 190.0);
    }

    #[test]
    fn wind_changes_world_position_without_camera_motion() {
        assert_eq!(cloud_world_coordinate(10.0, 0.0, 0.0), 10.0);
        assert_eq!(cloud_world_coordinate(10.0, 2.0, 0.0), 12.0);
    }
}
