use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::builtin_ids::CHISEL_TOOL_ID,
    gameplay::availability::WorldInteractionState,
    localization::{ActiveLanguage, UiLocalization},
    player::hotbar::PlayerHotbar,
    ui::typography,
    voxel::microblock::ChiselResolution,
};

#[derive(Component)]
struct ChiselHudText;

pub(super) struct ChiselHudPlugin;

impl Plugin for ChiselHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_chisel_hud)
            .add_systems(
                Update,
                sync_chisel_hud.run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn spawn_chisel_hud(mut commands: Commands) {
    commands.spawn((
        ChiselHudText,
        typography::hud(""),
        typography::tooltip_shadow(),
        TextLayout::new_with_justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(108),
            width: percent(100),
            ..default()
        },
        Visibility::Hidden,
        Pickable::IGNORE,
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn sync_chisel_hud(
    hotbar: Res<PlayerHotbar>,
    resolution: Res<ChiselResolution>,
    language: Res<ActiveLanguage>,
    translations: Res<UiLocalization>,
    interaction: WorldInteractionState,
    mut hud: Single<(&mut Text, &mut Visibility), With<ChiselHudText>>,
) {
    let active = interaction.available()
        && hotbar.item_at(hotbar.selected_slot()) == Some(CHISEL_TOOL_ID);
    let visibility = if active {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if *hud.1 != visibility {
        *hud.1 = visibility;
    }
    if !active {
        return;
    }

    let precision_key = match *resolution {
        ChiselResolution::Full => "chisel.precision.full",
        ChiselResolution::Thick => "chisel.precision.thick",
        ChiselResolution::Thin => "chisel.precision.thin",
        ChiselResolution::ExtraThin => "chisel.precision.extraThin",
    };
    let language = language.get();
    let text = translations
        .text(language, "hud.chisel")
        .replace("{precision}", translations.text(language, precision_key));
    if hud.0.0 != text {
        hud.0.0 = text;
    }
}
