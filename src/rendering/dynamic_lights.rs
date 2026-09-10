use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::block::BlockRegistry,
    player::{camera::GameplayCamera, hotbar::PlayerHotbar},
};

const MAX_HELD_LIGHT_INTENSITY: f32 = 90.0;
const HELD_LIGHT_RANGE: f32 = 8.0;
const HELD_LIGHT_RADIUS: f32 = 0.12;
const HELD_LIGHT_OFFSET: Vec3 = Vec3::new(0.32, -0.24, -0.52);

#[derive(Component)]
struct HeldDynamicLight;

pub struct DynamicLightsPlugin;

impl Plugin for DynamicLightsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (spawn_held_dynamic_light, sync_held_dynamic_light)
                .chain()
                .run_if(in_state(GameState::Gameplay)),
        );
    }
}

fn spawn_held_dynamic_light(
    mut commands: Commands,
    cameras: Query<Entity, Added<GameplayCamera>>,
    hotbar: Res<PlayerHotbar>,
    blocks: Res<BlockRegistry>,
) {
    let (intensity, visibility) = held_light_state(&hotbar, &blocks);

    for camera in &cameras {
        commands.entity(camera).with_children(|camera| {
            camera.spawn((
                HeldDynamicLight,
                PointLight {
                    color: Color::WHITE,
                    intensity,
                    range: HELD_LIGHT_RANGE,
                    radius: HELD_LIGHT_RADIUS,
                    shadow_maps_enabled: true,
                    ..default()
                },
                Transform::from_translation(HELD_LIGHT_OFFSET),
                visibility,
            ));
        });
    }
}

fn sync_held_dynamic_light(
    hotbar: Res<PlayerHotbar>,
    blocks: Res<BlockRegistry>,
    mut lights: Query<(&mut PointLight, &mut Visibility), With<HeldDynamicLight>>,
) {
    if !hotbar.is_changed() {
        return;
    }

    let (intensity, visibility) = held_light_state(&hotbar, &blocks);

    for (mut light, mut current_visibility) in &mut lights {
        if light.intensity != intensity {
            light.intensity = intensity;
        }
        if *current_visibility != visibility {
            *current_visibility = visibility;
        }
    }
}

fn held_light_state(hotbar: &PlayerHotbar, blocks: &BlockRegistry) -> (f32, Visibility) {
    let emission = hotbar
        .item_at(hotbar.selected_slot())
        .and_then(|block_id| blocks.get(block_id))
        .map_or(0, |block| block.light_emission)
        .min(15);
    let intensity = MAX_HELD_LIGHT_INTENSITY * emission as f32 / 15.0;
    let visibility = if emission > 0 {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    (intensity, visibility)
}
