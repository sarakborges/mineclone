use std::{collections::{HashMap, HashSet}, io};

use crate::{
    content::{
        block::BlockRegistry,
        creature::CreatureRegistry,
        day_night_cycle::DayNightCycleRegistry,
        dimension::DimensionRegistry,
        fluid::FluidRegistry,
        tool::ToolRegistry,
    },
    player::hotbar::{HOTBAR_SLOT_COUNT, INVENTORY_SLOT_COUNT},
};

use super::{invalid_data, snapshot::WorldSnapshot};
use crate::world::{
    fluid_updates::PendingFluidUpdates,
    new_world::is_valid_biome_size_multiplier,
};

#[derive(Clone, Copy)]
pub(crate) struct SaveRegistries<'a> {
    pub(crate) blocks: &'a BlockRegistry,
    pub(crate) fluids: &'a FluidRegistry,
    pub(crate) tools: &'a ToolRegistry,
    pub(crate) creatures: &'a CreatureRegistry,
    pub(crate) dimensions: &'a DimensionRegistry,
    pub(crate) cycles: &'a DayNightCycleRegistry,
}

impl SaveRegistries<'_> {
    pub(super) fn validate_playable(self, snapshot: &WorldSnapshot) -> io::Result<()> {
        let duration = self
            .dimensions
            .get(&snapshot.dimension_id)
            .and_then(|dimension| self.cycles.get(&dimension.day_night_cycle))
            .map(|cycle| cycle.day_duration_ticks);
        validate_playable(snapshot, duration, |id| {
            self.blocks.get(id).is_some() || self.tools.get(id).is_some()
        })?;
        PendingFluidUpdates::from_saved(&snapshot.fluid_updates, self.fluids)?;
        for creature in &snapshot.creatures {
            creature.validate(self.creatures)?;
        }
        Ok(())
    }

    pub(crate) fn owned_for_pruning(self) -> PruneRegistries {
        let valid_items = self
            .blocks
            .iter()
            .map(|block| block.id.clone())
            .chain(self.tools.iter().map(|tool| tool.id.clone()))
            .collect();
        let day_lengths = self
            .dimensions
            .iter()
            .filter_map(|dimension| {
                self.cycles
                    .get(&dimension.day_night_cycle)
                    .map(|cycle| (dimension.id.clone(), cycle.day_duration_ticks))
            })
            .collect();
        let mut creatures = CreatureRegistry::default();
        for definition in self.creatures.iter() {
            creatures.insert(definition.clone());
        }

        PruneRegistries {
            blocks: self.blocks.clone(),
            fluids: self.fluids.clone(),
            creatures,
            valid_items,
            day_lengths,
        }
    }
}

pub(crate) struct PruneRegistries {
    pub(super) blocks: BlockRegistry,
    pub(super) fluids: FluidRegistry,
    creatures: CreatureRegistry,
    valid_items: HashSet<String>,
    day_lengths: HashMap<String, u64>,
}

impl PruneRegistries {
    pub(crate) fn validate_playable(&self, snapshot: &WorldSnapshot) -> io::Result<()> {
        validate_playable(
            snapshot,
            self.day_lengths.get(&snapshot.dimension_id).copied(),
            |id| self.valid_items.contains(id),
        )?;
        PendingFluidUpdates::from_saved(&snapshot.fluid_updates, &self.fluids)?;
        for creature in &snapshot.creatures {
            creature.validate(&self.creatures)?;
        }
        Ok(())
    }
}

fn validate_playable(
    snapshot: &WorldSnapshot,
    duration: Option<u64>,
    valid_item: impl Fn(&str) -> bool,
) -> io::Result<()> {
    if duration.is_none_or(|ticks| ticks == 0 || snapshot.tick_in_day >= ticks) {
        return Err(invalid_data("saved dimension or world clock is invalid"));
    }
    if !is_valid_biome_size_multiplier(snapshot.biome_size_multiplier) {
        return Err(invalid_data("saved biome size multiplier is invalid"));
    }
    if let Some(player) = snapshot.player.as_ref() {
        if player
            .health
            .is_some_and(|health| !health.is_finite() || health < 0.0)
        {
            return Err(invalid_data("saved player health is invalid"));
        }
        if !player.yaw.is_finite() || !player.pitch.is_finite() {
            return Err(invalid_data("saved player look is invalid"));
        }
    }
    if snapshot.inventory.len() != INVENTORY_SLOT_COUNT {
        return Err(invalid_data("invalid inventory length"));
    }
    if snapshot.selected_hotbar_slot >= HOTBAR_SLOT_COUNT {
        return Err(invalid_data("invalid selected hotbar slot"));
    }
    for id in snapshot.inventory.iter().flatten() {
        if !valid_item(id) {
            return Err(invalid_data(format!("unknown inventory item ID: {id}")));
        }
    }
    Ok(())
}
