use bevy::{ecs::system::SystemParam, prelude::*};

use crate::world::WorldSeed;

use super::ChatState;

/// Phase-1 placeholder. The legacy locate implementation reconstructed worldgen
/// internals directly and was removed with the old generator. Phase 8 will
/// reconnect this command to the new biome/structure query capabilities.
#[derive(Resource, Default)]
pub(super) struct PendingLocate;

#[derive(SystemParam)]
pub(super) struct ChatLocateContext<'w> {
    _seed: Res<'w, WorldSeed>,
}

impl ChatLocateContext<'_> {
    pub(super) fn start(
        &mut self,
        _target_kind: &str,
        _id: &str,
        _variation: Option<usize>,
        _player_block: IVec3,
    ) -> String {
        "World generation rebuild in progress; /locate is temporarily unavailable.".to_owned()
    }
}

pub(super) fn poll_locate_task(
    _pending: ResMut<PendingLocate>,
    _chat: ResMut<ChatState>,
) {
}
