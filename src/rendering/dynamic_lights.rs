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
struct HeldDynamicLight {
    block_id: Option<&'static str>,
}

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
    let block_id = hotbar.item_at(hotbar.selected_slot());
    let (intensity, visibility) = held_light_state(block_id, &blocks);

    for camera in &cameras {
        commands.entity(camera).with_children(|camera| {
            camera.spawn((
                HeldDynamicLight { block_id },
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
    mut lights: Query<(&mut HeldDynamicLight, &mut PointLight, &mut Visibility)>,
) {
    let block_id = hotbar.item_at(hotbar.selected_slot());

    for (mut held, mut light, mut visibility) in &mut lights {
        if held.block_id == block_id {
            continue;
        }

        held.block_id = block_id;
        let (intensity, next_visibility) = held_light_state(block_id, &blocks);
        light.intensity = intensity;
        *visibility = next_visibility;
    }
}

fn held_light_state(
    block_id: Option<&'static str>,
    blocks: &BlockRegistry,
) -> (f32, Visibility) {
    let emission = block_id
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
