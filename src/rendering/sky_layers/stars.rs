use bevy::{
    light::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
};

use crate::{
    app::game_state::GameState,
    content::{
        day_night_cycle::DayNightCycleRegistry,
        day_night_phase::DayNightPhase,
        dimension::DimensionRegistry,
    },
    player::camera::GameplayCamera,
    world::{day_night::DayNightClock, dimension::CurrentDimension},
};

use super::state::SkyLayerVisualState;

const MAX_STARS: usize = 96;
const STAR_DISTANCE: f32 = 110.0;
const GOLDEN_ANGLE: f32 = 2.3999631;

#[derive(Resource)]
pub(super) struct StarAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

#[derive(Component)]
pub(super) struct Star {
    index: usize,
    direction: Vec3,
}

pub(super) fn setup_star_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Rectangle::new(1.0, 1.0));
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        fog_enabled: false,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    commands.insert_resource(StarAssets { mesh, material });
}

pub(super) fn spawn_stars(mut commands: Commands, assets: Res<StarAssets>) {
    for index in 0..MAX_STARS {
        let direction = star_direction(index);
        let size = 0.7 + hash01(index as u32) * 0.8;

        commands.spawn((
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(assets.material.clone()),
            Transform::from_scale(Vec3::splat(size)),
            Visibility::Hidden,
            NotShadowCaster,
            NotShadowReceiver,
            Star { index, direction },
            DespawnOnExit(GameState::Gameplay),
        ));
    }
}

pub(super) fn update_stars(
    visuals: Res<SkyLayerVisualState>,
    clock: Res<DayNightClock>,
    current_dimension: Res<CurrentDimension>,
    dimensions: Res<DimensionRegistry>,
    cycles: Res<DayNightCycleRegistry>,
    camera: Single<&GlobalTransform, With<GameplayCamera>>,
    assets: Res<StarAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut stars: Query<(&Star, &mut Transform, &mut Visibility)>,
) {
    let Some(dimension) = dimensions.get(&current_dimension.id) else {
        return;
    };
    let Some(cycle) = cycles.get(&dimension.day_night_cycle) else {
        return;
    };
    let sample = cycle.sample(clock.normalized_time);
    let time_factor = star_time_factor(sample.phase, sample.next_phase, sample.transition);
    let visible_count =
        (visuals.star_density * time_factor * MAX_STARS as f32).round() as usize;
    let camera_position = camera.translation();

    if let Some(mut material) = materials.get_mut(&assets.material) {
        let color = visuals.star_color;
        material.base_color = Color::srgba(color.r, color.g, color.b, time_factor);
    }

    for (star, mut transform, mut visibility) in &mut stars {
        if star.index >= visible_count || visuals.star_density <= 0.0 || time_factor <= 0.0 {
            *visibility = Visibility::Hidden;
            continue;
        }

        transform.translation = camera_position + star.direction * STAR_DISTANCE;
        transform.look_at(camera_position, Vec3::Y);
        *visibility = Visibility::Visible;
    }
}

fn star_direction(index: usize) -> Vec3 {
    let progress = (index as f32 + 0.5) / MAX_STARS as f32;
    let y = 0.08 + progress * 0.86;
    let horizontal = (1.0 - y * y).sqrt();
    let angle = index as f32 * GOLDEN_ANGLE;

    Vec3::new(angle.cos() * horizontal, y, angle.sin() * horizontal).normalize()
}

fn star_time_factor(phase: DayNightPhase, next_phase: DayNightPhase, transition: f32) -> f32 {
    match (phase, next_phase) {
        (DayNightPhase::Dusk, DayNightPhase::Night) => transition,
        (DayNightPhase::Night, DayNightPhase::Dawn) => 1.0 - transition,
        (DayNightPhase::Night, _) => 1.0,
        _ => 0.0,
    }
}

fn hash01(mut value: u32) -> f32 {
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^= value >> 16;
    value as f32 / u32::MAX as f32
}
