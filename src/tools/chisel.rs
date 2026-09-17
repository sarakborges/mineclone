//! Input state for the Chisel. The item is deliberately not registered in the
//! inventory until partial voxels are supported by meshing, collision and saves.

use bevy::prelude::*;

use crate::{
    content::builtin_ids::CHISEL_TOOL_ID,
    gameplay::availability::world_interaction_available,
    player::hotbar::PlayerHotbar,
};

/// Precision labels are ordered from the whole block to 1/8 of each axis.
/// The corresponding grids are 1x1x1, 2x2x2, 4x4x4 and 8x8x8.
#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ChiselResolution {
    #[default]
    Full,
    Thick,
    Thin,
    ExtraThin,
}

impl ChiselResolution {
    const fn next(self) -> Self {
        match self {
            Self::Full => Self::Thick,
            Self::Thick => Self::Thin,
            Self::Thin => Self::ExtraThin,
            Self::ExtraThin => Self::Full,
        }
    }
}

pub(super) struct ChiselPlugin;

impl Plugin for ChiselPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChiselResolution>().add_systems(
            Update,
            cycle_chisel_resolution.run_if(world_interaction_available),
        );
    }
}

fn cycle_chisel_resolution(
    keys: Res<ButtonInput<KeyCode>>,
    hotbar: Res<PlayerHotbar>,
    mut resolution: ResMut<ChiselResolution>,
) {
    if !keys.just_pressed(KeyCode::KeyR)
        || hotbar.item_at(hotbar.selected_slot()) != Some(CHISEL_TOOL_ID)
    {
        return;
    }

    *resolution = resolution.next();
}
