use bevy::{
    ecs::system::SystemParam,
    light::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
};

use crate::{
    app::game_state::GameState,
    content::day_night_phase::DayNightPhase,
    player::camera::GameplayCamera,
    world::current_context::DayNightContext,
};

use super::{assets::StarAssets, deterministic::hash01, state::SkyLayerVisualState};

const MAX_STARS: usize = 96;
const STAR_DISTANCE: f32 = 110.0;
const GOLDEN_ANGLE: f32 = 2.3999631;

#[derive(Component)]
pub(super) struct Star {
    index: usize,
    direction: Vec3,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct StarVisualSnapshot {
    visible_count: usize,
    color: Color,
}

pub(super) fn spawn_stars(mut commands: Commands, assets: Res<StarAssets>) {
    for index in 0..MAX_STARS {
        let direction = star_direction(index);
        let size = 0.7 + hash01(index as u32) * 0.8;
        let mut transform = Transform {
            translation: direction * STAR_DISTANCE,
            scale: Vec3::splat(size),
            ..default()
        };
        transform.look_at(Vec3::ZERO, Vec3::Y);

        commands.spawn((
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(assets.material.clone()),
            transform,
            Visibility::Hidden,
            NotShadowCaster,
            NotShadowReceiver,
            Star { index, direction },
            DespawnOnExit(GameState::Gameplay),
        ));
    }
}

#[derive(SystemParam)]
pub(super) struct StarScene<'w> {
    visuals: Res<'w, SkyLayerVisualState>,
    day_night: DayNightContext<'w>,
}

#[derive(SystemParam)]
pub(super) struct StarView<'w, 's> {
    camera: Single<'w, 's, (Entity, &'static GlobalTransform), With<GameplayCamera>>,
    assets: Res<'w, StarAssets>,
}

pub(super) fn update_stars(
    scene: StarScene,
    view: StarView,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut stars: Query<(&Star, &mut Transform, &mut Visibility)>,
    mut last_camera: Local<Option<(Entity, Vec3)>>,
    mut cached_visual: Local<Option<StarVisualSnapshot>>,
) {
    let StarView { camera, assets } = view;
    let (camera_entity, camera_transform) = camera.into_inner();
    let camera_position = camera_transform.translation();
    let camera_changed = last_camera.as_ref().is_none_or(|(entity, previous)| {
        *entity != camera_entity || *previous != camera_position
    });

    if !camera_changed && !scene.visuals.is_changed() && !scene.day_night.inputs_changed() {
        return;
    }

    let Some(sample) = scene.day_night.sample() else {
        return;
    };
    let time_factor = star_time_factor(sample.phase, sample.next_phase, sample.transition);
    let visible_count =
        (scene.visuals.star_density * time_factor * MAX_STARS as f32).round() as usize;
    let [red, green, blue] = scene.visuals.star_color.to_srgb();
    let color = Color::srgba(red, green, blue, time_factor);
    let visual_snapshot = StarVisualSnapshot {
        visible_count,
        color,
    };
    let visual_changed = cached_visual.as_ref() != Some(&visual_snapshot);

    if !camera_changed && !visual_changed {
        return;
    }

    if camera_changed {
        *last_camera = Some((camera_entity, camera_position));
    }

    if visual_changed {
        let material_color_changed = materials
            .get(&assets.material)
            .is_some_and(|material| material.base_color != visual_snapshot.color);
        if material_color_changed
            && let Some(mut material) = materials.get_mut(&assets.material)
        {
            material.base_color = visual_snapshot.color;
        }
    }

    for (star, mut transform, mut visibility) in &mut stars {
        if camera_changed {
            let translation = camera_position + star.direction * STAR_DISTANCE;
            if transform.translation != translation {
                transform.translation = translation;
            }
        }

        let next_visibility = if star.index < visible_count {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != next_visibility {
            *visibility = next_visibility;
        }
    }

    if visual_changed {
        *cached_visual = Some(visual_snapshot);
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
