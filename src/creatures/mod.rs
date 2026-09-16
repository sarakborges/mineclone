mod visual;

use bevy::prelude::*;

use crate::{
    app::game_state::GameState,
    content::creature::{CreatureCollider, CreatureRegistry},
    player::{PLAYER_EYE_HEIGHT, camera::GameplayCamera, find_safe_spawn_position},
    voxel::world::VoxelWorld,
};

use visual::{CreatureModel, attach_loaded_models};

/// The entity root owns position and collision; only its visual child is animated.
#[derive(Component)]
pub(crate) struct CreatureInstance {
    pub definition_id: String,
}

pub(crate) struct CreaturesPlugin;

impl Plugin for CreaturesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<visual::TintedCreatureMaterials>()
            .add_systems(OnEnter(GameState::Gameplay), spawn_preview_creatures)
            .add_systems(
                Update,
                attach_loaded_models.run_if(in_state(GameState::Gameplay)),
            );
    }
}

/// Temporary, explicitly opt-in content preview: world spawning/AI will be a
/// separate feature. All properties, including the model, come from JSON.
fn spawn_preview_creatures(
    mut commands: Commands,
    creatures: Res<CreatureRegistry>,
    players: Query<&Transform, With<GameplayCamera>>,
    world: Res<VoxelWorld>,
    asset_server: Res<AssetServer>,
) {
    let Some(player) = players.iter().next() else {
        warn!("creature previews skipped: player not spawned");
        return;
    };
    let mut previews: Vec<_> = creatures.iter().filter(|definition| definition.preview_spawn).collect();
    previews.sort_by(|left, right| left.id.cmp(&right.id));
    let base = player.translation.floor().as_ivec3();

    for (index, definition) in previews.into_iter().enumerate() {
        let preferred = IVec2::new(base.x + 3 + index as i32 * 2, base.z + 3);
        let Some(eye_position) = find_safe_spawn_position(&world, preferred, |_| true) else {
            warn!("no valid surface for creature preview {}", definition.id);
            continue;
        };
        let feet = eye_position - Vec3::Y * PLAYER_EYE_HEIGHT;
        let collider: CreatureCollider = definition.collider;
        let (min, max) = collider.bounds(feet);
        if !world.is_loaded_at(min.floor().as_ivec3())
            || !world.is_loaded_at(max.floor().as_ivec3())
        {
            continue;
        }

        commands.spawn((
            Name::new(format!("Creature: {}", definition.id)),
            CreatureInstance { definition_id: definition.id.clone() },
            CreatureModel(asset_server.load(definition.model.clone())),
            collider,
            Transform::from_translation(feet),
            DespawnOnExit(GameState::Gameplay),
        ));
    }
}
