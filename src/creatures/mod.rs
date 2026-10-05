mod combat;
mod lifecycle;
mod material;
mod metadata;
mod motion;
mod particles;
mod persistence;
mod spawn;
mod visual;
use crate::{
    app::{game_state::GameState, pause_state::PauseState, resource_systems::reset_resource},
    content::creature::CreatureCollider,
};
use bevy::prelude::*;

pub(crate) use combat::CreatureAttackRuntime;
pub(crate) use lifecycle::{CreatureDeathTimer, CreatureDespawnGrace};
use lifecycle::{despawn_dead_creatures, despawn_distant_creatures};
pub(crate) use material::apply_creature_material_overrides;
pub(crate) use metadata::EntityMetaTags;
use motion::move_creatures;
use particles::{emit_creature_particles, update_creature_particles};
pub(crate) use persistence::{PendingCreatureRestores, SavedCreature, sort_saved_creatures};
use spawn::restore_saved_creatures;
pub(crate) use spawn::{spawn_creature_at, spawn_creature_at_with_tags};
pub(crate) use visual::CreatureAnimationState;
use visual::{attach_loaded_models, sync_creature_animations, sync_creature_facing};

#[derive(Component)]
pub(crate) struct CreatureInstance {
    pub definition_id: String,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct CreatureTargetCollider(pub(crate) CreatureCollider);

pub(crate) struct CreaturesPlugin;

impl Plugin for CreaturesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<visual::TintedCreatureMaterials>()
            .init_resource::<particles::CreatureParticleAssets>()
            .init_resource::<PendingCreatureRestores>()
            .add_systems(
                OnEnter(GameState::StartingScreen),
                (
                    reset_resource::<PendingCreatureRestores>,
                    reset_resource::<visual::TintedCreatureMaterials>,
                ),
            )
            .add_systems(
                Update,
                restore_saved_creatures.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                despawn_distant_creatures
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running)),
            )
            .add_systems(
                Update,
                despawn_dead_creatures.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                attach_loaded_models.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                (
                    move_creatures,
                    emit_creature_particles,
                    update_creature_particles,
                )
                    .chain()
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
