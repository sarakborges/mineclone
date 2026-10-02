use bevy::prelude::*;

use crate::{
    app::crash_log::log_gameplay_event,
    content::{
        block::{BlockRegistry, BlockTint},
        builtin_ids::BIOME_TINT_METADATA_KEY,
    },
    gameplay::availability::world_interaction_available,
    player::{
        camera::GameplayCamera, game_mode::GameMode, hotbar::PlayerHotbar,
        viewmodel::ViewModelAnimation,
    },
    voxel::{cell::VoxelCell, edit::VoxelTopologyRuntime, read::VoxelRead},
};

use super::block::{BlockTargetingSet, TargetedBlock};

const ENCHANTED_FOREST_SPORES_ID: &str = "asteria:enchanted_forest_spores";
const ENCHANTED_FOREST_BIOME_ID: &str = "asteria:overworld/enchanted_forest";

pub(crate) struct BiomeTintInteractionPlugin;

impl Plugin for BiomeTintInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            apply_biome_tint_item
                .in_set(BlockTargetingSet::Interaction)
                .run_if(world_interaction_available),
        );
    }
}

fn apply_biome_tint_item(
    buttons: Res<ButtonInput<MouseButton>>,
    mut hotbar: ResMut<PlayerHotbar>,
    targeted: Res<TargetedBlock>,
    blocks: Res<BlockRegistry>,
    player: Single<&GameMode, With<GameplayCamera>>,
    mut runtime: VoxelTopologyRuntime,
    mut viewmodel_animation: ResMut<ViewModelAnimation>,
) {
    if !buttons.just_pressed(MouseButton::Right) {
        return;
    }

    let selected_slot = hotbar.selected_slot();
    let Some(item_id) = hotbar.item_at(selected_slot) else {
        return;
    };
    let Some(biome_id) = biome_tint_override_for_item(item_id) else {
        return;
    };
    let Some(hit) = targeted.0 else {
        return;
    };
    let Some(block) = blocks.get(hit.block_id) else {
        return;
    };
    let Some(cell) = runtime.read().cell_at(hit.voxel) else {
        return;
    };

    let BiomeTintCellUpdate::Applied(cell) =
        biome_tint_override_cell(cell, block.tint, biome_id)
    else {
        return;
    };

    if runtime.set_block(hit.voxel, Some(cell)).is_none() {
        log_gameplay_event(format!(
            "biome_tint.apply rejected item={} target={:?} biome={} reason=block_mutation_rejected",
            item_id, hit.voxel, biome_id
        ));
        return;
    }

    if *player.into_inner() == GameMode::Survival {
        let consumed = hotbar.consume_selected_item();
        debug_assert!(
            consumed,
            "successful survival biome tint application must consume the selected item"
        );
    }

    viewmodel_animation.play_place();
    log_gameplay_event(format!(
        "biome_tint.apply item={} target={:?} block={} biome={}",
        item_id, hit.voxel, hit.block_id, biome_id
    ));
}

fn biome_tint_override_for_item(item_id: &str) -> Option<&'static str> {
    match item_id {
        ENCHANTED_FOREST_SPORES_ID => Some(ENCHANTED_FOREST_BIOME_ID),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BiomeTintCellUpdate {
    Applied(VoxelCell),
    AlreadyApplied,
    UnsupportedBlock,
}

fn biome_tint_override_cell(
    cell: VoxelCell,
    tint: BlockTint,
    biome_id: &str,
) -> BiomeTintCellUpdate {
    if tint == BlockTint::None {
        return BiomeTintCellUpdate::UnsupportedBlock;
    }
    if cell.state(BIOME_TINT_METADATA_KEY) == Some(biome_id) {
        return BiomeTintCellUpdate::AlreadyApplied;
    }

    BiomeTintCellUpdate::Applied(cell.with_state(BIOME_TINT_METADATA_KEY, biome_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::texture_rotation::TextureRotation;

    fn test_cell() -> VoxelCell {
        VoxelCell::new("asteria:test", TextureRotation::Degrees0)
    }

    #[test]
    fn biome_tint_override_is_applied_to_tintable_blocks() {
        let BiomeTintCellUpdate::Applied(cell) = biome_tint_override_cell(
            test_cell(),
            BlockTint::Grass,
            ENCHANTED_FOREST_BIOME_ID,
        ) else {
            panic!("tintable block should accept biome tint override");
        };

        assert_eq!(
            cell.state(BIOME_TINT_METADATA_KEY),
            Some(ENCHANTED_FOREST_BIOME_ID)
        );
    }

    #[test]
    fn biome_tint_override_rejects_untinted_blocks() {
        assert_eq!(
            biome_tint_override_cell(
                test_cell(),
                BlockTint::None,
                ENCHANTED_FOREST_BIOME_ID,
            ),
            BiomeTintCellUpdate::UnsupportedBlock
        );
    }

    #[test]
    fn biome_tint_override_does_not_reapply_same_biome() {
        let cell = test_cell().with_state(BIOME_TINT_METADATA_KEY, ENCHANTED_FOREST_BIOME_ID);
        assert_eq!(
            biome_tint_override_cell(cell, BlockTint::Leaf, ENCHANTED_FOREST_BIOME_ID),
            BiomeTintCellUpdate::AlreadyApplied
        );
    }
}
