use bevy::{
    light::PointLightShadowMap,
    prelude::*,
    render::storage::ShaderBuffer,
};

use crate::{
    app::game_state::GameState,
    content::block::BlockRegistry,
    player::{camera::GameplayCamera, hotbar::PlayerHotbar},
    rendering::terrain_material::TerrainLightingBuffer,
};

const MAX_HELD_LIGHT_INTENSITY: f32 = 90.0;
const HELD_LIGHT_RANGE: f32 = 8.0;
const HELD_LIGHT_RADIUS: f32 = 0.12;
const HELD_LIGHT_OFFSET: Vec3 = Vec3::new(0.32, -0.24, -0.52);
const POINT_LIGHT_SHADOW_MAP_SIZE: usize = 512;

#[derive(Component)]
struct HeldDynamicLight {
    block_id: Option<&'static str>,
}

pub struct DynamicLightsPlugin;

impl Plugin for DynamicLightsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PointLightShadowMap {
            size: POINT_LIGHT_SHADOW_MAP_SIZE,
        })
        .add_systems(
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
    mut terrain_lighting: ResMut<TerrainLightingBuffer>,
    mut shader_buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let block_id = hotbar.item_at(hotbar.selected_slot());
    let (intensity, visibility) = held_light_state(block_id, &blocks);
    terrain_lighting.set_dynamic_light_enabled(
        &mut shader_buffers,
        visibility == Visibility::Visible,
    );

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
    mut terrain_lighting: ResMut<TerrainLightingBuffer>,
    mut shader_buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let selected_item_changed = hotbar.is_changed();
    let block_definitions_changed = blocks.is_changed();
    if !selected_item_changed && !block_definitions_changed {
        return;
    }

    let block_id = hotbar.item_at(hotbar.selected_slot());
    let (intensity, next_visibility) = held_light_state(block_id, &blocks);
    terrain_lighting.set_dynamic_light_enabled(
        &mut shader_buffers,
        next_visibility == Visibility::Visible,
    );

    for (mut held, mut light, mut visibility) in &mut lights {
        if held.block_id != block_id {
            held.block_id = block_id;
        }
        if light.intensity != intensity {
            light.intensity = intensity;
        }
        if *visibility != next_visibility {
            *visibility = next_visibility;
        }
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
