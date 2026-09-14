mod palette;

use bevy::prelude::*;

use crate::{
    app::{
        game_state::GameState,
        pause_state::PauseState,
        state_systems::{reset_next_state, reset_next_state_on_escape},
    },
    content::{block::BlockRegistry, builtin_ids::BRUSH_TOOL_ID},
    gameplay::availability::world_interaction_available,
    targeting::{ToolUse, ToolUseButton, block::BlockTargetingSet},
    voxel::edit::VoxelMutationRuntime,
};

use self::palette::{handle_palette_selection, spawn_brush_palette};

pub(crate) const DYED_PROPERTY_ID: &str = "dyed";
const DEFAULT_DYE_ID: &str = "red";

#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) enum BrushPaletteState {
    #[default]
    Closed,
    Open,
}

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
            selection: BrushSelection::Dye(DEFAULT_DYE_ID.to_owned()),
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
        app.init_state::<BrushPaletteState>()
            .init_resource::<BrushMode>()
            .add_systems(
                Update,
                handle_brush_use
                    .after(BlockTargetingSet::Interaction)
                    .run_if(world_interaction_available),
            )
            .add_systems(
                OnEnter(BrushPaletteState::Open),
                spawn_brush_palette.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                (
                    handle_palette_selection,
                    reset_next_state_on_escape::<BrushPaletteState>,
                )
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(BrushPaletteState::Open)),
            )
            .add_systems(
                OnEnter(PauseState::Paused),
                reset_next_state::<BrushPaletteState>.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                OnExit(GameState::Gameplay),
                reset_next_state::<BrushPaletteState>,
            );
    }
}

fn handle_brush_use(
    mut uses: MessageReader<ToolUse>,
    blocks: Res<BlockRegistry>,
    mode: Res<BrushMode>,
    mut runtime: VoxelMutationRuntime,
    mut next_palette: ResMut<NextState<BrushPaletteState>>,
) {
    for usage in uses.read() {
        if usage.tool_id != BRUSH_TOOL_ID {
            continue;
        }

        if usage.button == ToolUseButton::Right {
            next_palette.set(BrushPaletteState::Open);
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
