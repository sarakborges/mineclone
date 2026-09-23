use super::{
    builtin_ids::{BRUSH_TOOL_ID, CHISEL_TOOL_ID},
    loader::LoadedContent,
};

pub(super) fn validate_content(content: &LoadedContent) {
    for required_tool in [BRUSH_TOOL_ID, CHISEL_TOOL_ID] {
        assert!(
            content.tools.get(required_tool).is_some(),
            "missing required built-in tool definition: {required_tool}"
        );
    }

    content.player.validate_references(&content.attacks);

    for category in content.inventory_categories.iter() {
        category.validate_references(&content.blocks);
    }

    for block in content.blocks.iter() {
        assert!(
            content.layers.get(&block.id).is_none(),
            "content id {} cannot be both a block and a layer",
            block.id
        );
        block.validate_references(
            &content.inventory_categories,
            &content.secondary_properties,
            &content.tool_categories,
            &content.tools,
        );
    }

    for layer in content.layers.iter() {
        assert!(
            content.blocks.get(&layer.id).is_none(),
            "content id {} cannot be both a layer and a block",
            layer.id
        );
        assert!(
            content.tools.get(&layer.id).is_none(),
            "content id {} cannot be both a layer and a tool",
            layer.id
        );
        assert!(
            content.inventory_categories.get(&layer.category).is_some(),
            "layer {} references missing inventory category {}",
            layer.id,
            layer.category
        );
    }

    for tool in content.tools.iter() {
        assert!(
            content.blocks.get(&tool.id).is_none(),
            "content id {} cannot be both a block and a tool",
            tool.id
        );
        assert!(
            content.layers.get(&tool.id).is_none(),
            "content id {} cannot be both a tool and a layer",
            tool.id
        );
        assert!(
            content.inventory_categories.get(&tool.category).is_some(),
            "tool {} references missing inventory category {}",
            tool.id,
            tool.category
        );
        if let Some(category) = tool.mining.category() {
            assert!(
                content.tool_categories.get(category).is_some(),
                "tool {} references missing tool category {}",
                tool.id,
                category
            );
        }
    }

    for structure in content.structures.iter() {
        structure.validate_references(&content.blocks, &content.layers, &content.fluids);
    }

    for structure_set in content.structure_sets.iter() {
        assert!(
            !content.structures.resolves_reference(&structure_set.id),
            "content id {} cannot be both a structure/structure group and a structure set",
            structure_set.id
        );
        structure_set.validate_references(&content.structures);
    }

    for biome in content.biomes.iter() {
        biome.validate_material_references(&content.blocks);
        for spawn in &biome.creature_spawns {
            assert!(
                content.creatures.get(&spawn.creature).is_some(),
                "biome {} references missing creature spawn: {}",
                biome.id,
                spawn.creature
            );
        }
        biome.validate_structure_references(&content.structures, &content.structure_sets);
        if let Some(surface_fluid) = &biome.surface_fluid {
            assert!(
                content.fluids.id_of(surface_fluid.fluid_id()).is_some(),
                "biome {} surfaceFluid references missing fluid {}",
                biome.id,
                surface_fluid.fluid_id()
            );
        }
    }

    for dimension in content.dimensions.iter() {
        dimension.validate_biomes(&content.biomes);
        assert!(
            content.day_night_cycles.get(&dimension.day_night_cycle).is_some(),
            "dimension {} references missing day-night cycle {}",
            dimension.id,
            dimension.day_night_cycle
        );
        assert!(
            content.skies.get(&dimension.sky).is_some(),
            "dimension {} references missing sky {}",
            dimension.id,
            dimension.sky
        );
        dimension
            .hydrology
            .validate_references(&dimension.id, &content.biomes, &content.fluids);
    }
}
