use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState, settings_state::SettingsState},
    content::block::BlockRegistry,
    player::hotbar::{PlayerHotbar, PlayerHotbarSet},
    ui::{theme, typography},
};

pub struct CrosshairPlugin;

impl Plugin for CrosshairPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_crosshair)
            .add_systems(OnEnter(PauseState::Paused), hide_crosshair)
            .add_systems(OnEnter(SettingsState::Open), hide_crosshair)
            .add_systems(
                Update,
                update_rotation_hint
                    .after(PlayerHotbarSet::Selection)
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running)),
            )
            .add_systems(
                OnEnter(PauseState::Running),
                show_crosshair
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(SettingsState::Closed)),
            )
            .add_systems(
                OnEnter(SettingsState::Closed),
                show_crosshair
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running)),
            );
    }
}

#[derive(Component)]
struct CrosshairRoot;

#[derive(Component)]
struct RotationHint;

fn spawn_crosshair(mut commands: Commands) {
    commands
        .spawn((
            CrosshairRoot,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Visibility::Visible,
            Pickable::IGNORE,
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: px(18),
                    height: px(18),
                    position_type: PositionType::Relative,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|crosshair| {
                crosshair.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(2),
                        top: px(8),
                        width: px(14),
                        height: px(2),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(theme::TEXT_PRIMARY.with_alpha(0.92)),
                ));
                crosshair.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(8),
                        top: px(2),
                        width: px(2),
                        height: px(14),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(theme::TEXT_PRIMARY.with_alpha(0.92)),
                ));
                crosshair.spawn((
                    typography::caption("Press R to rotate block"),
                    TextLayout::justify(Justify::Center),
                    Node {
                        position_type: PositionType::Absolute,
                        top: px(26),
                        left: px(-111),
                        width: px(240),
                        ..default()
                    },
                    RotationHint,
                    Visibility::Hidden,
                ));
            });
        });
}

fn update_rotation_hint(
    hotbar: Res<PlayerHotbar>,
    blocks: Res<BlockRegistry>,
    mut hint: Single<&mut Visibility, With<RotationHint>>,
) {
    let rotatable = hotbar
        .item_at(hotbar.selected_slot())
        .and_then(|id| blocks.get(id))
        .is_some_and(|block| block.is_rotatable());
    let next = if rotatable {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    if **hint != next {
        **hint = next;
    }
}

fn hide_crosshair(mut roots: Query<&mut Visibility, With<CrosshairRoot>>) {
    for mut visibility in &mut roots {
        *visibility = Visibility::Hidden;
    }
}

fn show_crosshair(mut roots: Query<&mut Visibility, With<CrosshairRoot>>) {
    for mut visibility in &mut roots {
        *visibility = Visibility::Visible;
    }
}
