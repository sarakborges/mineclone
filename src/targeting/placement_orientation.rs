use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, resource_systems::reset_resource},
    content::{block::BlockDefinition, block::BlockRegistry, block_orientation::BlockOrientation},
    gameplay::availability::world_interaction_available,
    player::hotbar::{PlayerHotbar, PlayerHotbarSet},
};

use super::block::BlockTargetingSet;

#[derive(Resource, Default)]
pub(crate) struct PlacementOrientation {
    selected_slot: Option<usize>,
    orientation: BlockOrientation,
}

impl PlacementOrientation {
    pub(crate) fn for_block(
        &self,
        selected_slot: usize,
        block: &BlockDefinition,
    ) -> BlockOrientation {
        if self.selected_slot != Some(selected_slot) {
            return block.default_orientation();
        }

        if block.orientations.is_empty() {
            BlockOrientation::default()
        } else if block.orientations.contains(&self.orientation) {
            self.orientation
        } else {
            block.default_orientation()
        }
    }
}

pub(super) struct PlacementOrientationPlugin;

impl Plugin for PlacementOrientationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlacementOrientation>()
            .add_systems(
                OnEnter(GameState::Gameplay),
                reset_resource::<PlacementOrientation>,
            )
            .add_systems(
                Update,
                update_placement_orientation
                    .in_set(BlockTargetingSet::PlacementState)
                    .after(PlayerHotbarSet::Selection)
                    .run_if(world_interaction_available),
            );
    }
}

fn update_placement_orientation(
    keys: Res<ButtonInput<KeyCode>>,
    hotbar: Res<PlayerHotbar>,
    blocks: Res<BlockRegistry>,
    mut placement: ResMut<PlacementOrientation>,
) {
    let selected_slot = hotbar.selected_slot();
    let block = hotbar.item_at(selected_slot).and_then(|id| blocks.get(id));

    if placement.selected_slot != Some(selected_slot) {
        placement.selected_slot = Some(selected_slot);
        placement.orientation = block.map_or(BlockOrientation::default(), |block| {
            block.default_orientation()
        });
    }

    if !keys.just_pressed(KeyCode::KeyR) {
        return;
    }

    let Some(block) = block.filter(|block| block.is_rotatable()) else {
        return;
    };
    placement.orientation = block.next_orientation(placement.orientation);
}
