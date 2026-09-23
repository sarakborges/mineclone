mod palette;

use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, resource_systems::reset_resource},
    content::{
        block::BlockRegistry,
        builtin_ids::{BRUSH_TOOL_ID, DYED_PROPERTY_ID},
    },
    gameplay::{availability::world_interaction_available, modal::GameplayModalState},
    targeting::{ToolUse, ToolUseButton, block::BlockTargetingSet},
    voxel::edit::VoxelMutationRuntime,
};

use self::palette::{handle_palette_selection, spawn_brush_palette};

#[derive(Clone, Debug, PartialEq, Eq)]
enum BrushSelection {
    Clear,
    Dye(String),
}

#[derive(Resource, Clone, Debug)]
pub(crate) struct BrushMode {
    selection: BrushSelection,
}

impl Default for BrushMode {
    fn default() -> Self {
        Self {
            selection: BrushSelection::Clear,
        }
    }
}

impl BrushMode {
    pub(crate) fn dye_id(&self) -> Option<&str> {
        match &self.selection {
            BrushSelection::Clear => None,
            BrushSelection::Dye(id) => Some(id),
        }
    }
}

pub(super) struct BrushPlugin;

impl Plugin for BrushPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BrushMode>()
            .add_systems(
                OnEnter(GameState::Gameplay),
                reset_resource::<BrushMode>,
            )
            .add_systems(
                Update,
                handle_brush_use
                    .after(BlockTargetingSet::Interaction)
                    .run_if(world_interaction_available),
            )
            .add_systems(
                OnEnter(GameplayModalState::BrushPalette),
                spawn_brush_palette.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                handle_palette_selection
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(GameplayModalState::BrushPalette)),
            );
    }
}

fn handle_brush_use(
    mut uses: MessageReader<ToolUse>,
    blocks: Res<BlockRegistry>,
    mode: Res<BrushMode>,
    mut runtime: VoxelMutationRuntime,
    mut next_modal: ResMut<NextState<GameplayModalState>>,
) {
    for usage in uses.read() {
        if usage.tool_id != BRUSH_TOOL_ID {
            continue;
        }

        if usage.button == ToolUseButton::Right {
            next_modal.set(GameplayModalState::BrushPalette);
            continue;
        }

        let Some(hit) = usage.target else {
            continue;
        };
        let Some(block) = blocks.get(hit.block_id) else {
            continue;
        };
        if !block
            .secondary_properties
            .iter()
            .any(|property| property == DYED_PROPERTY_ID)
        {
            continue;
        }

        let Some(cell) = runtime.cell_at(hit.voxel) else {
            continue;
        };
        let current_dye = cell.secondary_property(DYED_PROPERTY_ID);
        let updated = match mode.dye_id() {
            Some(dye_id) => {
                if current_dye == Some(dye_id) {
                    continue;
                }
                cell.with_secondary_property(DYED_PROPERTY_ID, dye_id)
            }
            None => {
                if current_dye.is_none() {
                    continue;
                }
                cell.without_secondary_property(DYED_PROPERTY_ID)
            }
        };

        runtime.set_block(hit.voxel, Some(updated));
    }
}

#[cfg(test)]
mod tests {
    use super::BrushMode;

    #[test]
    fn brush_starts_without_a_selected_dye() {
        assert_eq!(BrushMode::default().dye_id(), None);
    }
}
