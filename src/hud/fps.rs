use bevy::prelude::*;

use crate::{app::game_state::GameState, ui::typography};

const FPS_UPDATE_INTERVAL_SECONDS: f32 = 0.25;

pub struct FpsHudPlugin;

impl Plugin for FpsHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_fps_hud)
            .add_systems(
                Update,
                update_fps_hud.run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Component, Default)]
struct FpsHud {
    elapsed_seconds: f32,
    frame_count: u32,
}

fn spawn_fps_hud(mut commands: Commands) {
    commands.spawn((
        typography::hud("0 FPS"),
        typography::tooltip_shadow(),
        FpsHud::default(),
        Node {
            position_type: PositionType::Absolute,
            right: px(16),
            bottom: px(16),
            ..default()
        },
        Pickable::IGNORE,
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn update_fps_hud(time: Res<Time>, fps_hud: Single<(&mut Text, &mut FpsHud)>) {
    let (mut text, mut fps) = fps_hud.into_inner();
    fps.elapsed_seconds += time.delta_secs();
    fps.frame_count += 1;

    if fps.elapsed_seconds < FPS_UPDATE_INTERVAL_SECONDS {
        return;
    }

    let frames_per_second = fps.frame_count as f32 / fps.elapsed_seconds.max(f32::EPSILON);
    text.0 = format!("{frames_per_second:.0} FPS");
    fps.elapsed_seconds = 0.0;
    fps.frame_count = 0;
}
