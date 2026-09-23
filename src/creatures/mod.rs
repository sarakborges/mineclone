mod material;
mod motion;
mod natural_spawn;
mod particles;
mod persistence;
mod spawn;
mod visual;
use bevy::prelude::*;
use crate::{
    app::{game_state::GameState, pause_state::PauseState, resource_systems::reset_resource},
    content::creature::CreatureCollider,
};

pub(crate) use material::apply_creature_material_overrides;
pub(crate) use motion::CreatureMotion;
use motion::move_creatures;
pub(crate) use persistence::{PendingCreatureRestores, SavedCreature};
use natural_spawn::natural_spawn_creatures;
use particles::{emit_creature_particles, update_creature_particles};
use spawn::restore_saved_creatures;
use visual::{attach_loaded_models, sync_creature_animations, sync_creature_facing};
pub(crate) use spawn::spawn_creature_at;
pub(crate) use visual::CreatureAnimationState;

/// The entity root owns position and collision; only its visual child is animated or rotated.
#[derive(Component)]
pub(crate) struct CreatureInstance {
    pub definition_id: String,
}

#[derive(Component)]
pub(crate) struct CreatureDeathTimer(pub(crate) Timer);

/// Visual targeting can be larger than the physics AABB. This prevents a ray
/// from visually entering a creature before gameplay considers it targeted.
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
                restore_saved_creatures
                    .run_if(in_state(GameState::Gameplay))
                    .before(natural_spawn_creatures),
            )
            .add_systems(Update, natural_spawn_creatures.run_if(in_state(GameState::Gameplay)).run_if(in_state(PauseState::Running)))
            .add_systems(Update, despawn_dead_creatures.run_if(in_state(GameState::Gameplay)))
            .add_systems(Update, attach_loaded_models.run_if(in_state(GameState::Gameplay)))

            .add_systems(
                Update,
                (move_creatures, emit_creature_particles, update_creature_particles)
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

fn despawn_dead_creatures(
    time: Res<Time>,
    mut commands: Commands,
    mut dead: Query<(Entity, &mut CreatureDeathTimer)>,
) {
    for (entity, mut timer) in &mut dead {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            commands.entity(entity).despawn();
        }
    }
}
