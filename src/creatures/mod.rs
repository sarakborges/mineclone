mod motion;
mod visual;

use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    content::creature::CreatureRegistry,
    localization::Language,
};

use motion::{CreatureMotion, move_creatures};
use visual::{CreatureModel, attach_loaded_models, sync_creature_animations, sync_creature_facing};

/// The entity root owns position and collision; only its visual child is animated or rotated.
#[derive(Component)]
pub(crate) struct CreatureInstance {
    pub definition_id: String,
}

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct EntityHealth {
    current: f32,
    max: f32,
}

impl EntityHealth {
    pub(crate) fn new(max: f32) -> Self {
        assert!(max.is_finite() && max > 0.0, "entity health must be positive and finite");
        Self { current: max, max }
    }

    pub(crate) fn current(self) -> f32 { self.current }
    pub(crate) fn max(self) -> f32 { self.max }
    pub(crate) fn damage(&mut self, amount: f32) -> bool {
        self.current = (self.current - amount.max(0.0)).max(0.0);
        self.current <= 0.0
    }
}

pub(crate) struct CreaturesPlugin;

impl Plugin for CreaturesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<visual::TintedCreatureMaterials>()
            // Creatures only spawn through explicit gameplay commands, never world creation.
            .add_systems(Update, attach_loaded_models.run_if(in_state(GameState::Gameplay)))
            .add_systems(
                Update,
                move_creatures
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running)),
            )
            .add_systems(
                PostUpdate,
                (sync_creature_facing, sync_creature_animations)
                    .chain()
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

/// The chat preflights world occupancy and relocates the player before calling
/// this function. The exact feet position is preserved; this function never
/// searches neighboring slots or mutates the terrain.
pub(crate) fn spawn_creature_at(
    commands: &mut Commands,
    definitions: &CreatureRegistry,
    asset_server: &AssetServer,
    language: Language,
    id: &str,
    feet: Vec3,
) -> Result<String, String> {
    let definition = definitions
        .get(id)
        .ok_or_else(|| format!("Unknown creature id: {id}"))?;
    let name = definition.name.text(language).to_owned();
    commands.spawn((
        Name::new(name.clone()),
        CreatureInstance {
            definition_id: definition.id.clone(),
        },
        CreatureModel(asset_server.load(definition.model.clone())),
        CreatureMotion::default(),
        definition.collider,
        Transform::from_translation(feet),
        Visibility::default(),
        DespawnOnExit(GameState::Gameplay),
    ));
    Ok(name)
}
